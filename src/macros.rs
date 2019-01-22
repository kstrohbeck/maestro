#[macro_export]
macro_rules! pop {
    ($hash:ident[$key:expr]) => {
        $hash.remove(&::yaml_rust::Yaml::from_str($key))
    };
    ($hash:ident[$key:expr] as String) => {
        $hash
            .remove(&::yaml_rust::Yaml::from_str($key))
            .and_then(::yaml_rust::Yaml::into_string)
    };
}
