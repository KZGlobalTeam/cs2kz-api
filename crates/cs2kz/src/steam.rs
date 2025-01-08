define_id_type! {
    /// An identifier for Steam Workshop items.
    #[derive(sqlx::Type)]
    #[sqlx(transparent)]
    pub struct WorkshopId(u32);
}
