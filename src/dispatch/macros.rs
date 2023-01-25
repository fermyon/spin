#[macro_export]
macro_rules! __match_trait {
    ($enum:ident, $type:ident, [$($variant:ident),*], $value:ident, $block:block) => {
        match $enum { $($type::$variant($value) => $block,)* }
    };
}

#[macro_export]
macro_rules! match_trait {
    (
        match $enum:ident {
            $type:ident::($($variant:ident)|*)($value:ident) => $block:block
        }
    ) => {
        __match_trait!($enum, $type, [$($variant),*], $value, $block);
    };
}

#[macro_export]
macro_rules! __impl_trait {
    ([$($attrs:meta),*], $trait:ident, $type:ident, $function:ident, $value:ident, [$($variant:ident),*], $arg:ident, $argtype:ty, $ret:ty, $block:block) => {
        $(#[$attrs])*
        impl $trait for $type {
            async fn $function(&self, $arg: $argtype) -> $ret {
                __match_trait!(self, Self, [$($variant),*], $value, $block)
            }
        }
    };
}

#[macro_export]
macro_rules! impl_trait {
    (
        $(#[$attrs:meta])*
        impl $trait:ident for $type:ident {
            async fn $function:ident($value:ident: $($variant:ident)|*, $arg:ident: $argtype:ty) -> $ret:ty $block:block
        }
    ) => {
        __impl_trait!([$($attrs),*], $trait, $type, $function, $value, [$($variant),*], $arg, $argtype, $ret, $block);
    }
}

#[macro_export]
macro_rules! __type_enum {
    ($($enum_attrs:meta)*, $enum:ident, $($($var_attrs:meta)*, $variant:ident, $type:ident),*) => {
        $(#[$enum_attrs])*
        enum $enum {
            $(
                $(#[$var_attrs])*
                $variant($type)
            ),*
        }
    }
}

#[macro_export]
macro_rules! type_enum {
    (
        $(#[$enum_attrs:meta])*
        enum $enum:ident {
            $(
                $(#[$var_attrs:meta])*
                $variant:ident($type:ident)
            ),*
        }
    ) => {
        __type_enum!($($enum_attrs)*, $enum, $($($var_attrs)*, $variant, $type),*);
    }
}

#[macro_export]
macro_rules! trait_enum {
        ($(#[$attrs:meta])*
        enum $enum:ident: $trait:ty {
            $(
                $(#[$var_attrs:meta])*
                $variant:ident($type:ident)
        ),*
        }
    ) => {
        __type_enum!($($attrs)*, $enum, $($($var_attrs)*, $variant, $type),*);
        impl_dispatch!($enum::{$($type),*});
    }
}


#[macro_export]
macro_rules! match_action  {
    ($value:ident[$action:ident]$(.$ex:ident)*) => {
        match $action {
            Action::Run => $value.run()$(.$ex)*,
            Action::Help => $value.help()$(.$ex)*
        }
    };
}

#[macro_export]
macro_rules! impl_dispatch {
    ($type:ident::{$($variant:ident),*}) => {
        __impl_trait!(
            [async_trait::async_trait(?Send)],
            Dispatch, 
            $type, 
            dispatch,
            value,
            [$($variant),*],
            action,
            &Action,
            Result<()>,
            { match_action!(value[action].await) }
        );
    };
}