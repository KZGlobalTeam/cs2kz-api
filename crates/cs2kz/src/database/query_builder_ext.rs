use std::{fmt, mem};

use super::QueryBuilder;

pub trait QueryBuilderExt<'args, Driver: sqlx::Database = super::Driver> {
    fn with_filter<R>(&mut self, f: impl FnOnce(FilterState<'_, 'args, Driver>) -> R) -> R;
}

impl<'args, Driver: sqlx::Database> QueryBuilderExt<'args, Driver> for QueryBuilder<'args, Driver> {
    fn with_filter<R>(&mut self, f: impl FnOnce(FilterState<'_, 'args, Driver>) -> R) -> R {
        f(FilterState::new(self))
    }
}

#[derive(Debug)]
pub struct FilterState<'query_builder, 'query_builder_args, Driver: sqlx::Database = super::Driver>
{
    query_builder: &'query_builder mut QueryBuilder<'query_builder_args, Driver>,
    next_keyword: FilterKeyword,
}

#[derive(Debug, Clone, Copy)]
enum FilterKeyword {
    Where,
    And,
}

impl<'query_builder, 'query_builder_args, Driver: sqlx::Database>
    FilterState<'query_builder, 'query_builder_args, Driver>
{
    fn new(query_builder: &'query_builder mut QueryBuilder<'query_builder_args, Driver>) -> Self {
        Self {
            query_builder,
            next_keyword: FilterKeyword::Where,
        }
    }

    pub fn filter(&mut self, sql: impl fmt::Display) -> &mut Self {
        self.query_builder.push(match self.next_keyword.next() {
            FilterKeyword::Where => " WHERE ",
            FilterKeyword::And => " AND ",
        });

        self.query_builder.push(sql);

        self
    }
}

impl FilterKeyword {
    fn next(&mut self) -> Self {
        mem::replace(self, Self::And)
    }
}
