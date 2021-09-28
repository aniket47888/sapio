// Copyright Judica, Inc 2021
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! macros for making defining Sapio contracts less verbose.

use core::any::TypeId;
pub use paste::paste;
use schemars::schema::RootSchema;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

/// The declare macro is used to declare the list of pathways in a Contract trait impl.
/// formats for calling are:
/// ```ignore
/// declare!{then, a,...}
/// declare!{finish, a,...}
/// declare!{updatable<X>, a,...}
/// /// because of a quirk in stable rust, non updatable
/// /// is required if no updatable<X> declaration is made
/// /// nightly rust does not require this, but it is availble
/// /// for compatibility
/// declare!{non updatable}
/// ```
#[macro_export]
macro_rules! declare {
    {then $(,$a:expr)*} => {
        /// binds the list of `ThenFunc`'s to this impl.
        /// Any fn() which returns None is ignored (useful for type-level state machines)
        const THEN_FNS: &'static [fn() -> Option<$crate::contract::actions::ThenFunc<'static, Self>>] = &[$($a,)*];
    };
    [state $i:ty]  => {
        type StatefulArguments = $i;
    };

    [state]  => {
        /// Due to type system limitations, all `FinishOrFuncs` for a Contract type must share a
        /// parameter pack type. If Nightly, trait default types allowed.
        #[cfg(feature = "nightly")]
        type StatefulArguments = ();
        /// Due to type system limitations, all `FinishOrFuncs` for a Contract type must share a
        /// parameter pack type. If stable, no default type allowed.
        #[cfg(not(feature = "nightly"))]
        type StatefulArguments;
    };
    {updatable<$($i:ty)?> $(,$a:expr)*} => {
        /// binds the list of `FinishOrFunc`'s to this impl.
        /// Any fn() which returns None is ignored (useful for type-level state machines)
        const FINISH_OR_FUNCS: &'static [fn() -> Option<Box<dyn $crate::contract::actions::CallableAsFoF<Self, Self::StatefulArguments>>>] = &[$($a,)*];
        declare![state $($i)?];
    };
    {non updatable} => {
        #[cfg(not(feature = "nightly"))]
        declare![state ()];
    };
    {finish $(,$a:expr)*} => {
        /// binds the list of `Gurard`'s to this impl as unlocking conditions.
        /// `Guard`s only need to be bound if it is desired that they are
        /// sufficient to unlock funds, a `Guard` should not be bound if it is
        /// intended to be used with a `ThenFunc`.
        /// Any fn() which returns None is ignored (useful for type-level state machines)
        const FINISH_FNS: &'static [fn() -> Option<$crate::contract::actions::Guard<Self>>] = &[$($a,)*];
    };


}

/// The then macro is used to define a `ThenFunc`
/// formats for calling are:
/// ```ignore
/// /// A Guarded CTV Function
/// then!(guarded_by: [guard_1, ... guard_n] fn name(self, ctx) {/*Result<Box<Iterator<TransactionTemplate>>>*/} );
/// /// A Conditional CTV Function
/// then!(compile_if: [compile_if_1, ... compile_if_n] fn name(self, ctx) {/*Result<Box<Iterator<TransactionTemplate>>>*/} );
/// /// An Unguarded CTV Function
/// then!(fn name(self, ctx) {/*Result<Box<Iterator<TransactionTemplate>>>*/} );
/// /// Null Implementation
/// then!(name);
/// ```
#[macro_export]
macro_rules! then {
    {
        $(#[$meta:meta])*
        $name:ident
    } => {

        $crate::contract::macros::paste!{

            $(#[$meta])*
            fn [<THEN_ $name>](&self, _ctx:$crate::contract::Context)-> $crate::contract::TxTmplIt
            {
                unimplemented!();
            }
            $(#[$meta])*
            fn $name<'a>() -> Option<$crate::contract::actions::ThenFunc<'a, Self>> {None}
        }
    };
    {
        $(#[$meta:meta])*
        compile_if: $conditional_compile_list:tt
        guarded_by: $guard_list:tt
        fn $name:ident($s:ident, $ctx:ident)
        $b:block
    } => {

        $crate::contract::macros::paste!{

            $(#[$meta])*
            fn [<THEN_ $name>](&$s, $ctx:$crate::contract::Context) -> $crate::contract::TxTmplIt
            $b
            $(#[$meta])*
            fn $name<'a>() -> Option<$crate::contract::actions::ThenFunc<'a, Self>>{
                Some($crate::contract::actions::ThenFunc{
                    guard: &$guard_list,
                    conditional_compile_if: &$conditional_compile_list,
                    func: Self::[<THEN_ $name>]
                })
            }
        }
    };
    {
        $(#[$meta:meta])*
        fn $name:ident($s:ident, $ctx:ident) $b:block
    } => {
        then!{
            $(#[$meta])*
            compile_if: []
            guarded_by: []
            fn $name($s, $ctx) $b
        }
    };

    {
        $(#[$meta:meta])*
        guarded_by: $guard_list:tt
        fn $name:ident($s:ident, $ctx:ident) $b:block
    } => {
        then!{
            $(#[$meta])*
            compile_if: []
            guarded_by: $guard_list
            fn $name($s, $ctx) $b }
    };

    {
        $(#[$meta:meta])*
        compile_if: $conditional_compile_list:tt
        fn $name:ident($s:ident, $ctx:ident) $b:block
    } => {
        then!{
            $(#[$meta])*
            compile_if: $conditional_compile_list
            guarded_by: []
            fn $name($s, $ctx) $b }
    };

}

lazy_static::lazy_static! {
static ref SCHEMA_MAP: Mutex<HashMap<TypeId, Arc<RootSchema>>> =
Mutex::new(HashMap::new());
}
/// `get_schema_for` returns a cached RootSchema for a given type.  this is
/// useful because we might expect to generate the same RootSchema many times,
/// and they can use a decent amount of memory.
pub fn get_schema_for<T: schemars::JsonSchema + 'static + Sized>(
) -> Arc<schemars::schema::RootSchema> {
    SCHEMA_MAP
        .lock()
        .unwrap()
        .entry(TypeId::of::<T>())
        .or_insert_with(|| Arc::new(schemars::schema_for!(T)))
        .clone()
}

/// Internal Helper for finish! macro, not to be used directly.
#[macro_export]
macro_rules! web_api {
    {web{},$name:ident,$type:ty} => {
        $crate::contract::macros::paste!{
            const [<FINISH_API_FOR_ $name >] : Option<std::sync::Arc<$crate::schemars::schema::RootSchema>> = Some($crate::contract::macros::get_schema_for::<$type>());
        }
    };
    {$name:ident,$type:ty} => {
        $crate::contract::macros::paste!{
            const [<FINISH_API_FOR_ $name >] : Option<std::sync::Arc<$crate::schemars::schema::RootSchema>> = None;
        }
    }
}
/// The finish macro is used to define a `FinishFunc` or a `FinishOrFunc`
/// formats for calling are:
/// ```ignore
/// /// A Guarded CTV Function
/// finish!(guarded_by: [guard_1, ... guard_n] fn name(self, ctx, o) {/*Result<Box<Iterator<TransactionTemplate>>>*/} );
/// /// A Conditional CTV Function
/// finish!(compile_if: [compile_if_1, ... compile_if_n] guarded_by: [guard_1, ..., guard_n] fn name(self, ctx, o) {/*Result<Box<Iterator<TransactionTemplate>>>*/} );
/// /// Null Implementation
/// finish!(name);
/// ```
/// Unlike a `then!`, `finish!` must always have guards.
#[macro_export]
macro_rules! finish {
    {
        $(#[$meta:meta])*
        $(web$web_enable:block)?
        $name:ident<$arg_type:ty>
    } => {
        $crate::contract::macros::paste!{
            web_api!($(web$web_enable,)* $name,$arg_type);
            $(#[$meta])*
            fn [<FINISH_ $name>](&self, _ctx:$crate::contract::Context, _o: $arg_type)-> $crate::contract::TxTmplIt
            {
                unimplemented!();
            }
            $(#[$meta])*
            fn $name<'a>() ->
            Option<Box<dyn
            $crate::contract::actions::CallableAsFoF<Self, <Self as $crate::contract::Contract>::StatefulArguments>>>
            {
                None
            }
        }
    };
    {
        $(#[$meta:meta])*
        $(web$web_enable:block)?
        compile_if: $conditional_compile_list:tt
        guarded_by: $guard_list:tt
        coerce_args: $coerce_args:ident
        fn $name:ident($s:ident, $ctx:ident, $o:ident : $arg_type:ty)
        $b:block
    } => {

        $crate::contract::macros::paste!{
            web_api!($(web$web_enable,)* $name,$arg_type);
            $(#[$meta])*
            fn [<FINISH_ $name>](&$s, $ctx:$crate::contract::Context, $o: $arg_type) -> $crate::contract::TxTmplIt
            $b
            $(#[$meta])*
            fn $name<'a>() -> Option<Box<dyn
            $crate::contract::actions::CallableAsFoF<Self, <Self as $crate::contract::Contract>::StatefulArguments>>>
            {
                let f = $crate::contract::actions::FinishOrFunc{
                    coerce_args: $coerce_args,
                    guard: &$guard_list,
                    conditional_compile_if: &$conditional_compile_list,
                    func: Self::[<FINISH_ $name>],
                    schema: Self::[<FINISH_API_FOR_ $name >],
                    name: std::stringify!($name).into()
                };
                Some(Box::new(f))
            }
        }
    };
    {
        $(#[$meta:meta])*
        $(web$web_enable:block)?
        guarded_by: $guard_list:tt
        coerce_args: $coerce_args:ident
        fn $name:ident($s:ident, $ctx:ident, $o:ident:$arg_type:ty) $b:block
    } => {
        finish!{
            $(#[$meta])*
            $(web$web_enable)*
            compile_if: []
            guarded_by: $guard_list
            coerce_args: $coerce_args
            fn $name($s, $ctx, $o:$arg_type) $b }
    };
}

/// The guard macro is used to define a `Guard`. Guards may be cached or uncached.
/// formats for calling are:
/// ```ignore
/// guard!(fn name(self, ctx) {/*Clause*/})
/// /// The guard should only be invoked once
/// guard!(cached fn name(self, ctx) {/*Clause*/})
/// ```
#[macro_export]
macro_rules! guard {
    {
        $(#[$meta:meta])*
        $name:ident} => {
            $crate::contract::macros::paste!{
                $(#[$meta])*
                fn [<GUARD_ $name>](&self, _ctx:$crate::contract::Context) -> $crate::sapio_base::Clause {
                    unimplemented!();
                }
                $(#[$meta])*
                fn $name() -> Option<$crate::contract::actions::Guard<Self>> {
                    None
                }
            }
     };
    {
        $(#[$meta:meta])*
        fn $name:ident($s:ident, $ctx:ident) $b:block} => {
            $crate::contract::macros::paste!{
                $(#[$meta])*
                fn [<GUARD_ $name>](&$s, $ctx:$crate::contract::Context) -> $crate::sapio_base::Clause
                $b
                $(#[$meta])*
                fn  $name() -> Option<$crate::contract::actions::Guard<Self>> {
                    Some($crate::contract::actions::Guard::Fresh(Self::[<GUARD_ $name>]))
                }

            }
        };
    {
        $(#[$meta:meta])*
        cached
        fn $name:ident($s:ident, $ctx:ident) $b:block} => {
            $crate::contract::macros::paste!{
                $(#[$meta])*
                fn [<GUARD_ $name>](&$s, $ctx:$crate::contract::Context) -> $crate::sapio_base::Clause
                $b

                $(#[$meta])*
                fn $name() -> Option<$crate::contract::actions::Guard<Self>> {
                    Some($crate::contract::actions::Guard::Cache(Self::[<GUARD_ $name>]))
                }
            }
        };
}

/// The compile_if macro is used to define a `ConditionallyCompileIf`.
/// formats for calling are:
/// ```ignore
/// compile_if!(fn name(self, ctx) {/*ConditionallyCompileType*/})
/// ```
#[macro_export]
macro_rules! compile_if {
    {
        $(#[$meta:meta])*
        $name:ident
    } => {
            $crate::contract::macros::paste!{
                $(#[$meta])*
                fn [<COMPILE_IF $name>](&self, _ctx: $crate::contract::Context) -> $crate::contract::actions::ConditionalCompileType {
                    unimplemented!()
                }
                $(#[$meta])*
                fn $name() -> Option<$crate::contract::actions::ConditionallyCompileIf<Self>> {
                    None
                }
            }
     };
    {
        $(#[$meta:meta])*
        fn $name:ident($s:ident, $ctx:ident) $b:block
    } => {
            $crate::contract::macros::paste!{
                $(#[$meta])*
                fn [<COMPILE_IF $name>](&$s, $ctx: $crate::contract::Context) -> $crate::contract::actions::ConditionalCompileType
                $b
                $(#[$meta])*
                fn $name() -> Option<$crate::contract::actions::ConditionallyCompileIf<Self>> {
                    Some($crate::contract::actions::ConditionallyCompileIf::Fresh(Self::[<COMPILE_IF $name>]))
                }
            }
        };
}
