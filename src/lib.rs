use async_trait::async_trait;

pub trait Manifest {
    type Overrides;

    fn manifest(overrides: Self::Overrides) -> Self;
}

#[async_trait]
pub trait Persist: Manifest + Sized {
    type Conn;
    type Err;

    async fn persist(conn: &mut Self::Conn) -> Result<Self, Self::Err>;
}

#[cfg(test)]
mod tests {
    use derive_builder::Builder;
    use rusqlite::{params, Connection};

    use super::*;

    #[derive(Debug, Builder, PartialEq, Eq)]
    struct Movie {
        pub title: String,
        pub year: u32,
    }

    impl Manifest for Movie {
        type Overrides = MovieBuilder;

        fn manifest(builder: Self::Overrides) -> Self {
            Self {
                title: builder.title.unwrap_or("Inception".into()),
                year: builder.year.unwrap_or(2010),
            }
        }
    }

    #[async_trait]
    impl Persist for Movie {
        type Conn = Connection;
        type Err = rusqlite::Error;

        async fn persist(conn: &mut Self::Conn) -> Result<Self, Self::Err> {
            let movie = Movie::manifest(MovieBuilder::default());

            conn.execute(
                "
                    insert into movie (title, year) values ($1, $2)
                ",
                params![movie.title, movie.year],
            )?;

            Ok(movie)
        }
    }

    #[test]
    fn manifest_works() {
        let movie = Movie::manifest(MovieBuilder::default());

        assert_eq!(
            movie,
            Movie {
                title: "Inception".into(),
                year: 2010
            }
        )
    }

    #[test]
    fn manifest_works_with_overrides() {
        let movie = Movie::manifest({
            let mut movie = MovieBuilder::default();
            movie.title("The Social Network".into());
            movie
        });

        assert_eq!(
            movie,
            Movie {
                title: "The Social Network".into(),
                year: 2010
            }
        )
    }

    #[tokio::test]
    async fn persist_works() -> Result<(), Box<dyn std::error::Error>> {
        let mut conn = Connection::open(":memory:")?;

        conn.execute(
            r#"
                create table if not exists movie (
                    id integer primary key,
                    title text not null unique,
                    year integer not null
                );
            "#,
            (),
        )?;

        let movie = Movie::persist(&mut conn).await?;

        assert_eq!(
            movie,
            Movie {
                title: "Inception".into(),
                year: 2010
            }
        );

        let persisted_movie = conn.query_row(
            "
                select title, year from movie where title = $1
            ",
            [movie.title.clone()],
            |row| {
                Ok(Movie {
                    title: row.get(0).unwrap(),
                    year: row.get(1).unwrap(),
                })
            },
        )?;

        assert_eq!(movie, persisted_movie);

        Ok(())
    }
}
