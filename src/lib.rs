use async_trait::async_trait;

pub trait Manifest {
    type Overrides;

    fn manifest(overrides: Self::Overrides) -> Self;
}

#[async_trait]
pub trait Persist: Manifest {
    async fn persist() -> Self;
}

#[cfg(test)]
mod tests {
    use derive_builder::Builder;

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
}
