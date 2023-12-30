use std::any::{Any, TypeId};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::{manifest, persist, Persist};

pub fn association<T: Persist + 'static>(associations: &mut Associations<T::Context>) -> T {
    let entity = manifest::<T>();

    associations.persist::<T, _>(move |conn| {
        Box::pin(async move {
            let entity = persist::<T>(conn).await.map_err(|_| "failed to persist")?;

            Ok(entity)
        })
    });

    entity
}

pub(crate) struct AnyAssociation<Context> {
    entity_type: TypeId,
    pub(crate) persist: Box<
        dyn FnOnce(
            Arc<Context>,
        ) -> Pin<
            Box<dyn Future<Output = Result<Box<dyn Any>, Box<dyn std::error::Error>>>>,
        >,
    >,
}

pub struct Associations<Context> {
    pub(crate) associations: Vec<AnyAssociation<Context>>,
}

impl<Context: 'static> Associations<Context> {
    pub fn new() -> Self {
        Self {
            associations: Vec::new(),
        }
    }

    pub(crate) fn persist<
        T: 'static,
        F: FnOnce(
                Arc<Context>,
            )
                -> Pin<Box<dyn Future<Output = Result<T, Box<dyn std::error::Error>>>>>
            + 'static,
    >(
        &mut self,
        persist: F,
    ) {
        self.associations.push(AnyAssociation {
            entity_type: TypeId::of::<T>(),
            persist: Box::new(|conn| {
                Box::pin(async move {
                    let value = persist(conn).await?;

                    Ok(Box::new(value) as Box<dyn Any>)
                })
                    as Pin<
                        Box<dyn Future<Output = Result<Box<dyn Any>, Box<dyn std::error::Error>>>>,
                    >
            }),
        });
    }
}
