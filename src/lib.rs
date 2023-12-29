#![doc = include_str!("../README.md")]

pub trait Manifest {
    type Overrides: Default;

    fn manifest(overrides: Self::Overrides) -> Self;
}

pub trait Persist: Manifest + Sized {
    type Conn;
    type Err;

    #[allow(async_fn_in_trait)]
    async fn persist(conn: &mut Self::Conn, entity: Self) -> Result<Self, Self::Err>;
}

#[inline(always)]
pub fn manifest<T: Manifest>() -> T {
    manifest_with(T::Overrides::default())
}

pub fn manifest_with<T: Manifest>(overrides: T::Overrides) -> T {
    T::manifest(overrides)
}

#[inline(always)]
pub async fn persist<T: Persist>(conn: &mut T::Conn) -> Result<T, T::Err> {
    persist_with(conn, T::Overrides::default()).await
}

pub async fn persist_with<T: Persist>(
    conn: &mut T::Conn,
    overrides: T::Overrides,
) -> Result<T, T::Err> {
    let entity = manifest_with(overrides);

    T::persist(conn, entity).await
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

    impl Persist for Movie {
        type Conn = Connection;
        type Err = rusqlite::Error;

        async fn persist(conn: &mut Self::Conn, movie: Self) -> Result<Self, Self::Err> {
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
        let movie: Movie = manifest();

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
        let movie: Movie = manifest_with({
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

        let movie: Movie = persist(&mut conn).await?;

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

    #[tokio::test]
    async fn persist_works_with_overrides() -> Result<(), Box<dyn std::error::Error>> {
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

        let movie: Movie = persist_with(&mut conn, {
            let mut movie = MovieBuilder::default();
            movie.title("The Social Network".into());
            movie
        })
        .await?;

        assert_eq!(
            movie,
            Movie {
                title: "The Social Network".into(),
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
