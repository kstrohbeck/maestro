//! Helper macros.

macro_rules! field {
    ( $map:ident, $field:ident ) => {{
        if $field.is_some() {
            return Err(serde::de::Error::duplicate_field(stringify!($field)));
        }
        $field = Some($map.next_value()?);
    }};
    ( $field:ident $blk:block ) => {{
        if $field.is_some() {
            return Err(serde::de::Error::duplicate_field(stringify!($field)));
        }
        $field = Some($blk);
    }};
}

macro_rules! ser_field {
    ( $state:expr, $field:literal, $val:expr ) => {
        if let Some(val) = $val {
            $state.serialize_field($field, &val)?;
        }
    };
}
