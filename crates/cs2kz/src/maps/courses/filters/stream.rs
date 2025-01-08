use std::pin::Pin;
use std::task::{Poll, ready};

use futures_util::Stream;

use crate::database;
use crate::maps::courses::filters::GetCourseFiltersError;
use crate::maps::{CourseFilter, CourseFilters};

#[pin_project]
#[derive(Debug)]
pub(super) struct GetCourseFiltersStream<S> {
    #[pin]
    #[debug(skip)]
    stream: S,
    current: Option<CourseFilter>,
}

impl<S: Stream<Item = database::Result<CourseFilter>>> GetCourseFiltersStream<S> {
    pub(super) fn new(stream: S) -> Self {
        Self { stream, current: None }
    }
}

impl<S: Stream<Item = database::Result<CourseFilter>>> Stream for GetCourseFiltersStream<S> {
    type Item = Result<CourseFilters, GetCourseFiltersError>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut me = self.project();

        let current = loop {
            if let Some(current) = me.current.take() {
                break current;
            }

            match ready!(me.stream.as_mut().poll_next(cx)) {
                // no current filter AND stream is done -> we're done
                None => return Poll::Ready(None),
                // no current filter, stream returned an error -> yield error
                Some(Err(error)) => return Poll::Ready(Some(Err(error.into()))),
                // no current filter, stream yielded next filter -> set current and try again
                Some(Ok(next)) => {
                    *me.current = Some(next);
                },
            }
        };

        Poll::Ready(Some(match ready!(me.stream.as_mut().poll_next(cx)) {
            // we have a current filter, but stream is done -> bug
            None => panic!("odd amount of filters in the database"),
            // we have a current filter, but stream yielded an error -> yield error
            Some(Err(error)) => Err(error.into()),
            // we have a current filter, stream yielded next filter
            Some(Ok(next)) => Ok(CourseFilters { vanilla: current, classic: next }),
        }))
    }
}
