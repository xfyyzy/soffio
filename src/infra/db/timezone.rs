use std::fmt;

use chrono_tz::Tz;
use sqlx::encode::IsNull;
use sqlx::postgres::{PgArgumentBuffer, PgTypeInfo, PgValueRef};
use sqlx::{Decode, Encode, Postgres, Type};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DbTimeZone(pub Tz);

impl From<Tz> for DbTimeZone {
    fn from(value: Tz) -> Self {
        Self(value)
    }
}

impl From<DbTimeZone> for Tz {
    fn from(value: DbTimeZone) -> Self {
        value.0
    }
}

impl fmt::Display for DbTimeZone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Type<Postgres> for DbTimeZone {
    fn type_info() -> PgTypeInfo {
        <String as Type<Postgres>>::type_info()
    }

    fn compatible(ty: &PgTypeInfo) -> bool {
        <String as Type<Postgres>>::compatible(ty)
    }
}

impl<'q> Encode<'q, Postgres> for DbTimeZone {
    fn encode_by_ref(
        &self,
        buf: &mut PgArgumentBuffer,
    ) -> Result<IsNull, sqlx::error::BoxDynError> {
        <&str as Encode<Postgres>>::encode(self.0.name(), buf)
    }

    fn size_hint(&self) -> usize {
        <&str as Encode<Postgres>>::size_hint(&self.0.name())
    }
}

impl<'r> Decode<'r, Postgres> for DbTimeZone {
    fn decode(value: PgValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let raw = <&str as Decode<Postgres>>::decode(value)?;
        raw.parse::<Tz>()
            .map(DbTimeZone)
            .map_err(|err| format!("invalid timezone '{}': {err}", raw).into())
    }
}
