use cs2kz::servers::ServerId;
use cs2kz::users::UserId;
use utoipa::openapi::schema::{self, KnownFormat, SchemaFormat, SchemaType};
use utoipa::openapi::{Object, RefOr, Schema};
use utoipa::{PartialSchema, ToSchema};

#[derive(ToSchema)]
pub struct Paginated<T> {
    /// The total number of values available for fetching.
    ///
    /// Different endpoints have different hard-limits on how many values they will return at
    /// a time. They usually also have an `offset` query parameter you can use to fetch the next
    /// set of values. You can use `total` to infer when you can stop making requests.
    #[allow(dead_code)]
    total: u64,

    /// The values returned for this request.
    #[allow(dead_code)]
    values: Vec<T>,
}

#[derive(ToSchema, serde::Serialize)]
#[serde(rename_all = "kebab-case", tag = "type", content = "id")]
pub enum BannedBy {
    /// The ban was issued by the Anti-Cheat on a CS2 server.
    #[schema(value_type = u16)]
    Server(ServerId),

    /// The ban was issued by an admin.
    #[schema(value_type = SteamId64)]
    Admin(UserId),
}

schema_type!(Limit => {
    Schema::Object(
        Object::builder()
            .schema_type(SchemaType::Type(schema::Type::Integer))
            .format(Some(SchemaFormat::KnownFormat(KnownFormat::UInt64)))
            .build(),
    )
});

schema_type!(Offset => {
    Schema::Object(
        Object::builder()
            .schema_type(SchemaType::Type(schema::Type::Integer))
            .format(Some(SchemaFormat::KnownFormat(KnownFormat::Int64)))
            .build(),
    )
});

schema_type!(SteamId => {
    Schema::Object(
        Object::builder()
            .description(Some("a SteamID"))
            .schema_type(SchemaType::Type(schema::Type::String))
            .examples(["STEAM_1:1:161178172"])
            .build(),
    )
});

schema_type!(SteamId64 => {
    Schema::Object(
        Object::builder()
            .description(Some("a 64-bit SteamID"))
            .schema_type(SchemaType::Type(schema::Type::String))
            .format(Some(SchemaFormat::KnownFormat(KnownFormat::UInt64)))
            .examples(["76561198282622073"])
            .build(),
    )
});

schema_type!(Timestamp => {
    Schema::Object(
        Object::builder()
            .description(Some("a UTC timestamp"))
            .schema_type(SchemaType::Type(schema::Type::String))
            .format(Some(SchemaFormat::KnownFormat(KnownFormat::DateTime)))
            .examples(["1970-01-01T00:00:00Z"])
            .build(),
    )
});

schema_type!(GitRevision => {
    Schema::Object(
        Object::builder()
            .description(Some("a git revision"))
            .schema_type(SchemaType::Type(schema::Type::String))
            .min_length(Some(40))
            .max_length(Some(40))
            .examples(["24bfd2242fc46340c95574468d78af687dea0e93"])
            .build(),
    )
});

schema_type!(Permissions => {
    Schema::Array(
        Object::builder()
            .description(Some("user permission"))
            .schema_type(SchemaType::Type(schema::Type::String))
            .enum_values(Some(["user-permissions", "servers", "map-pool", "player-bans"]))
            .examples(["servers", "player-bans"])
            .to_array_builder()
            .build(),
    )
});

schema_type!(ServerHost => {
    Schema::Object(
        Object::builder()
            .description(Some("an IPv4/IPv6 address or domain"))
            .schema_type(SchemaType::Type(schema::Type::String))
            .format(Some(SchemaFormat::KnownFormat(KnownFormat::Hostname)))
            .examples(["255.255.255.255", "::1", "example.org"])
            .build(),
    )
});

schema_type!(AccessKey => {
    Schema::Object(
        Object::builder()
            .description(Some("an opaque access key"))
            .schema_type(SchemaType::Type(schema::Type::String))
            .format(Some(SchemaFormat::KnownFormat(KnownFormat::Ulid)))
            .examples(["01JG9X9ZMAKCNXH19VMXZ7BC08"])
            .build(),
    )
});

schema_type!(MapState => {
    Schema::Object(
        Object::builder()
            .schema_type(SchemaType::Type(schema::Type::String))
            .enum_values(Some(["invalid", "in-testing", "approved"]))
            .build(),
    )
});

schema_type!(CourseFilterState => {
    Schema::Object(
        Object::builder()
            .schema_type(SchemaType::Type(schema::Type::String))
            .enum_values(Some(["unranked", "pending", "ranked"]))
            .build(),
    )
});

schema_type!(CourseFilterTier => {
    Schema::Object(
        Object::builder()
            .schema_type(SchemaType::Type(schema::Type::String))
            .enum_values(Some([
                "very-easy",
                "easy",
                "medium",
                "advanced",
                "hard",
                "very-hard",
                "extreme",
                "death",
                "unfeasible",
                "impossible",
            ]))
            .build(),
    )
});

schema_type!(Mode => {
    Schema::Object(
        Object::builder()
            .schema_type(SchemaType::Type(schema::Type::String))
            .enum_values(Some(["vanilla", "classic"]))
            .build(),
    )
});

schema_type!(Styles => {
    Schema::Array(
        Object::builder()
            .schema_type(SchemaType::Type(schema::Type::String))
            .enum_values(Some(["auto-bhop"]))
            .to_array_builder()
            .build(),
    )
});

schema_type!(JumpType => {
    Schema::Object(
        Object::builder()
            .schema_type(SchemaType::Type(schema::Type::String))
            .enum_values(Some([
                "long-jump",
                "bhop",
                "multi-bhop",
                "weird-jump",
                "ladder-jump",
                "ladderhop",
                "jumpbug",
                "fall",
            ]))
            .build(),
    )
});

schema_type!(BanReason => {
    Schema::Object(
        Object::builder()
            .schema_type(SchemaType::Type(schema::Type::String))
            .enum_values(Some(["macro", "auto-bhop", "auto-strafe"]))
            .build(),
    )
});

macro schema_type($name:ident => $impl:block) {
    pub struct $name;

    impl PartialSchema for $name {
        fn schema() -> RefOr<Schema> {
            { $impl }.into()
        }
    }

    impl ToSchema for $name {}
}
