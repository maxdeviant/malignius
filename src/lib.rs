#![doc = include_str!("../README.md")]

mod associations;
mod sequence;

use std::sync::Arc;

pub use associations::*;
pub use sequence::*;

pub trait Manifest {
    type Context;
    type Overrides: Default;

    fn manifest(overrides: Self::Overrides) -> (Self, Associations<Self::Context>)
    where
        Self: Sized;
}

pub trait Persist: Manifest {
    type Err;

    #[allow(async_fn_in_trait)]
    async fn persist(ctx: &Self::Context, entity: Self) -> Result<Self, Self::Err>
    where
        Self: Sized;
}

#[inline(always)]
pub fn manifest<T: Manifest>() -> T {
    manifest_with(T::Overrides::default())
}

pub fn manifest_with<T: Manifest>(overrides: T::Overrides) -> T {
    let (entity, _) = T::manifest(overrides);
    entity
}

#[inline(always)]
pub async fn persist<T: Persist>(ctx: Arc<T::Context>) -> Result<T, T::Err> {
    persist_with(ctx, T::Overrides::default()).await
}

pub async fn persist_with<T: Persist>(
    ctx: Arc<T::Context>,
    overrides: T::Overrides,
) -> Result<T, T::Err> {
    let (entity, associations) = T::manifest(overrides);

    for association in associations.associations {
        (association.persist)(ctx.clone()).await.unwrap();
    }

    T::persist(&ctx, entity).await
}

#[cfg(test)]
mod tests {
    use derive_builder::Builder;
    use rusqlite::{params, Connection};

    use super::*;

    struct TestContext {
        pub conn: Connection,
    }

    #[derive(Debug, Builder, PartialEq, Eq)]
    struct Movie {
        pub title: String,
        pub year: u32,
    }

    impl Manifest for Movie {
        type Context = TestContext;
        type Overrides = MovieBuilder;

        fn manifest(overrides: Self::Overrides) -> (Self, Associations<Self::Context>) {
            (
                Self {
                    title: overrides.title.unwrap_or("Inception".into()),
                    year: overrides.year.unwrap_or(2010),
                },
                Associations::new(),
            )
        }
    }

    impl Persist for Movie {
        type Err = rusqlite::Error;

        async fn persist(ctx: &Self::Context, movie: Self) -> Result<Self, Self::Err> {
            ctx.conn.execute(
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
        let conn = Connection::open(":memory:")?;

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

        let ctx = Arc::new(TestContext { conn });

        let movie: Movie = persist(ctx.clone()).await?;

        assert_eq!(
            movie,
            Movie {
                title: "Inception".into(),
                year: 2010
            }
        );

        let persisted_movie = ctx.conn.query_row(
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
        let conn = Connection::open(":memory:")?;

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

        let ctx = Arc::new(TestContext { conn });

        let movie: Movie = persist_with(ctx.clone(), {
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

        let persisted_movie = ctx.conn.query_row(
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

    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
    struct AuthorId(u32);

    #[derive(Debug, Builder, PartialEq, Eq)]
    struct Author {
        pub id: AuthorId,
        pub name: String,
    }

    impl Manifest for Author {
        type Context = TestContext;
        type Overrides = AuthorBuilder;

        fn manifest(overrides: Self::Overrides) -> (Self, Associations<Self::Context>) {
            (
                Self {
                    id: overrides.id.unwrap_or(AuthorId(1)),
                    name: overrides.name.unwrap_or("Author 1".into()),
                },
                Associations::new(),
            )
        }
    }

    impl Persist for Author {
        type Err = rusqlite::Error;

        async fn persist(ctx: &Self::Context, author: Self) -> Result<Self, Self::Err> {
            ctx.conn.execute(
                "
                    insert into author (id, name) values ($1, $2)
                ",
                params![author.id.0, author.name],
            )?;

            Ok(author)
        }
    }

    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
    struct PostId(u32);

    #[derive(Debug, Builder, PartialEq, Eq)]
    struct Post {
        pub id: PostId,
        pub author_id: AuthorId,
        pub title: String,
    }

    impl Manifest for Post {
        type Context = TestContext;
        type Overrides = PostBuilder;

        fn manifest(overrides: Self::Overrides) -> (Self, Associations<Self::Context>) {
            let mut associations = Associations::new();

            let author_id = overrides
                .author_id
                .unwrap_or_else(|| association::<Author>(&mut associations).id);

            (
                Self {
                    id: overrides.id.unwrap_or(PostId(1)),
                    author_id,
                    title: overrides.title.unwrap_or("Post 1".into()),
                },
                associations,
            )
        }
    }

    impl Persist for Post {
        type Err = rusqlite::Error;

        async fn persist(ctx: &Self::Context, post: Self) -> Result<Self, Self::Err> {
            ctx.conn.execute(
                "
                    insert into post (id, author_id, title) values ($1, $2, $3)
                ",
                params![post.id.0, post.author_id.0, post.title],
            )?;

            Ok(post)
        }
    }

    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
    struct CommentId(u32);

    #[derive(Debug, Builder, PartialEq, Eq, Clone)]
    struct Comment {
        pub id: CommentId,
        pub post_id: PostId,
        pub username: String,
    }

    impl Manifest for Comment {
        type Context = TestContext;
        type Overrides = CommentBuilder;

        fn manifest(overrides: Self::Overrides) -> (Self, Associations<Self::Context>) {
            let mut associations = Associations::new();

            let post_id = overrides
                .post_id
                .unwrap_or_else(|| association::<Post>(&mut associations).id);
            (
                Self {
                    id: overrides.id.unwrap_or(CommentId(1)),
                    post_id,
                    username: overrides.username.unwrap_or("user1".into()),
                },
                associations,
            )
        }
    }

    impl Persist for Comment {
        type Err = rusqlite::Error;

        async fn persist(ctx: &Self::Context, comment: Self) -> Result<Self, Self::Err> {
            ctx.conn.execute(
                "
                    insert into comment (id, post_id, username) values ($1, $2, $3)
                ",
                params![comment.id.0, comment.post_id.0, comment.username],
            )?;

            Ok(comment)
        }
    }

    #[tokio::test]
    async fn persist_works_with_an_entity_hierarchy() -> Result<(), Box<dyn std::error::Error>> {
        let conn = Connection::open(":memory:")?;

        conn.pragma_update(None, "foreign_keys", "on")?;

        conn.execute_batch(
            r#"
                create table if not exists author (
                    id integer primary key,
                    name text not null unique
                );

                create table if not exists post (
                    id integer primary key,
                    author_id integer not null references author (id),
                    title text not null
                );

                create table if not exists comment (
                    id integer primary key,
                    post_id integer not null references post (id),
                    username text not null
                );
            "#,
        )?;

        let ctx = Arc::new(TestContext { conn });

        let comment: Comment = persist(ctx.clone()).await?;

        let persisted_comments = {
            let mut stmt = ctx
                .conn
                .prepare("select id, post_id, username from comment")?;
            let persisted_comments = stmt
                .query_map([], |row| {
                    Ok(Comment {
                        id: CommentId(row.get(0).unwrap()),
                        post_id: PostId(row.get(1).unwrap()),
                        username: row.get(2).unwrap(),
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;

            persisted_comments
        };

        assert_eq!(persisted_comments, vec![comment.clone()]);

        let persisted_posts = {
            let mut stmt = ctx.conn.prepare("select id, author_id, title from post")?;
            let persisted_posts = stmt
                .query_map([], |row| {
                    Ok(Post {
                        id: PostId(row.get(0).unwrap()),
                        author_id: AuthorId(row.get(1).unwrap()),
                        title: row.get(2).unwrap(),
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;

            persisted_posts
        };

        assert_eq!(
            persisted_posts,
            vec![Post {
                id: comment.post_id,
                author_id: AuthorId(1),
                title: "Post 1".into()
            }]
        );

        Ok(())
    }
}
