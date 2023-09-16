use async_trait::async_trait;

pub trait Manifest {
    fn manifest() -> Self;
}

#[async_trait]
pub trait Persist: Manifest {
    async fn persist() -> Self;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct Movie {
        pub title: String,
        pub year: u32,
    }

    impl Manifest for Movie {
        fn manifest() -> Self {
            Self {
                title: "Inception".into(),
                year: 2010,
            }
        }
    }

    #[test]
    fn manifest_works() {
        let movie = Movie::manifest();

        assert_eq!(
            movie,
            Movie {
                title: "Inception".into(),
                year: 2010
            }
        )
    }
}
