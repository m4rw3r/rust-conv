/*!
This module defines the various error types that can be produced by a failed conversion.

In addition, it also defines some extension traits to make working with failable conversions more ergonomic (see the `Unwrap*` traits).
*/

use std::any::Any;
use std::error::Error;
use std::fmt::{self, Debug, Display};
use misc::{Saturated, InvalidSentinel, SignedInfinity};

macro_rules! Desc {
    (
        ($desc:expr)
        pub struct $name:ident<$t:ident> $_body:tt;
    ) => {
        impl<$t> Display for $name<$t> {
            fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
                write!(fmt, $desc)
            }
        }

        impl<$t> Error for $name<$t> where $t: Any {
            fn description(&self) -> &str {
                $desc
            }
        }
    };
}

macro_rules! DummyDebug {
    (
        () pub enum $name:ident<$t:ident> {
            $(#[doc=$_doc:tt] $vname:ident($_vpay:ident),)+
        }
    ) => {
        impl<$t> Debug for $name<$t> {
            fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
                let msg = match *self {
                    $($name::$vname(_) => stringify!($vname),)+
                };
                write!(fmt, concat!(stringify!($name), "::{}(..)"), msg)
            }
        }
    };

    (
        () pub struct $name:ident<$t:ident>(pub $_pay:ident);
    ) => {
        impl<$t> Debug for $name<$t> {
            fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
                write!(fmt, concat!(stringify!($name), "(..)"))
            }
        }
    };
}

macro_rules! EnumDesc {
    (
        ($($vname:ident => $vdesc:expr,)+) 
        pub enum $name:ident $_body:tt
    ) => {
        impl Display for $name {
            fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
                write!(fmt, "{}",
                    match *self { $($name::$vname => $vdesc,)+ })
            }
        }

        impl Error for $name {
            fn description(&self) -> &str {
                match *self { $($name::$vname => $vdesc,)+ }
            }
        }
    };

    (
        ($($vname:ident => $vdesc:expr,)+) 
        pub enum $name:ident<$t:ident> $_body:tt
    ) => {
        impl<$t> Display for $name<$t> {
            fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
                write!(fmt, "{}",
                    match *self { $($name::$vname(..) => $vdesc,)+ })
            }
        }

        impl<$t> Error for $name<$t> where $t: Any {
            fn description(&self) -> &str {
                match *self { $($name::$vname(..) => $vdesc,)+ }
            }
        }
    };
}

macro_rules! FromName {
    (
        ($fname:ident)
        pub enum $name:ident<$t:ident> $_body:tt
    ) => {
        impl<$t> From<$fname<$t>> for $name<$t> {
            #[inline]
            fn from(e: $fname<$t>) -> Self {
                $name::$fname(e.into_inner())
            }
        }
    };

    (
        ($fname:ident<$t:ident>)
        pub enum $name:ident $_body:tt
    ) => {
        impl<$t> From<$fname<$t>> for $name {
            #[inline]
            fn from(_: $fname<$t>) -> Self {
                $name::$fname
            }
        }
    };
}

macro_rules! FromNoError {
    (
        () pub enum $name:ident $_body:tt
    ) => {
        impl From<NoError> for $name {
            #[inline]
            fn from(_: NoError) -> Self {
                panic!(concat!("cannot convert NoError into ", stringify!($name)))
            }
        }
    };

    (
        () pub enum $name:ident<$t:ident> $_body:tt
    ) => {
        impl<$t> From<NoError> for $name<$t> {
            fn from(_: NoError) -> Self {
                panic!(concat!("cannot convert NoError into ", stringify!($name)))
            }
        }
    };

    (
        () pub struct $name:ident<$t:ident> $_body:tt;
    ) => {
        impl<$t> From<NoError> for $name<$t> {
            fn from(_: NoError) -> Self {
                panic!(concat!("cannot convert NoError into ", stringify!($name)))
            }
        }
    };
}

macro_rules! FromRemap {
    (
        ($from:ident($($vname:ident),+))
        pub enum $name:ident $_body:tt
    ) => {
        impl From<$from> for $name {
            #[inline]
            fn from(e: $from) -> Self {
                match e {
                    $($from::$vname => $name::$vname,)+
                }
            }
        }
    };

    (
        ($from:ident<$t:ident>($($vname:ident),+))
        pub enum $name:ident $_body:tt
    ) => {
        impl<$t> From<$from<$t>> for $name {
            #[inline]
            fn from(e: $from<$t>) -> Self {
                match e {
                    $($from::$vname(..) => $name::$vname,)+
                }
            }
        }
    };

    (
        ($from:ident($($vname:ident),+))
        pub enum $name:ident<$t:ident> $_body:tt
    ) => {
        impl<$t> From<$from<$t>> for $name<$t> {
            #[inline]
            fn from(e: $from<$t>) -> Self {
                match e {
                    $($from::$vname(v) => $name::$vname(v),)+
                }
            }
        }
    };
}

macro_rules! IntoInner {
    (
        () pub enum $name:ident<$t:ident> {
            $(#[doc=$_doc:tt] $vname:ident($_vpay:ident),)+
        }
    ) => {
        impl<$t> $name<$t> {
            /// Returns the value stored in this error.
            #[inline]
            pub fn into_inner(self) -> $t {
                match self { $($name::$vname(v))|+ => v }
            }
        }
    };

    (
        () pub struct $name:ident<$t:ident>(pub $_pay:ident);
    ) => {
        impl<$t> $name<$t> {
            /// Returns the value stored in this error.
            #[inline]
            pub fn into_inner(self) -> $t {
                self.0
            }
        }
    };
}

custom_derive!{
    /**
    A general error enumeration that subsumes all other conversion errors.

    This exists primarily as a "catch-all" for reliably unifying various different kinds of conversion errors.
    */
    #[derive(
        Copy, Clone, Eq, PartialEq, Ord, PartialOrd,
        IntoInner, DummyDebug, FromNoError,
        EnumDesc(
            Underflow => "conversion resulted in underflow",
            Overflow => "conversion resulted in overflow",
            Unrepresentable => "could not convert unrepresentable value",
        ),
        FromName(Unrepresentable),
        FromName(Underflow),
        FromName(Overflow),
        FromRemap(RangeError(Underflow, Overflow))
    )]
    pub enum GeneralError<T> {
        /// Input underflowed the target type.
        Underflow(T),

        /// Input overflowed the target type.
        Overflow(T),

        /// Input was not representable in the target type.
        Unrepresentable(T),
    }
}

impl<T> From<FloatError<T>> for GeneralError<T> {
    #[inline]
    fn from(e: FloatError<T>) -> GeneralError<T> {
        use self::FloatError as F;
        use self::GeneralError as G;
        match e {
            F::Underflow(v) => G::Underflow(v),
            F::Overflow(v) => G::Overflow(v),
            F::NotANumber(v) => G::Unrepresentable(v),
        }
    }
}

custom_derive! {
    /**
    A general error enumeration that subsumes all other conversion errors, but discards all input payloads the errors may be carrying.

    This exists primarily as a "catch-all" for reliably unifying various different kinds of conversion errors, and between different input types.
    */
    #[derive(
        Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug,
        FromNoError,
        EnumDesc(
            Underflow => "conversion resulted in underflow",
            Overflow => "conversion resulted in overflow",
            Unrepresentable => "could not convert unrepresentable value",
        ),
        FromName(Unrepresentable<T>),
        FromName(Underflow<T>),
        FromName(Overflow<T>),
        FromRemap(RangeErrorKind(Underflow, Overflow)),
        FromRemap(RangeError<T>(Underflow, Overflow)),
        FromRemap(GeneralError<T>(Underflow, Overflow, Unrepresentable))
    )]
    pub enum GeneralErrorKind {
        /// Input underflowed the target type.
        Underflow,

        /// Input overflowed the target type.
        Overflow,

        /// Input was not representable in the target type.
        Unrepresentable,
    }
}

impl<T> From<FloatError<T>> for GeneralErrorKind {
    #[inline]
    fn from(e: FloatError<T>) -> GeneralErrorKind {
        use self::FloatError as F;
        use self::GeneralErrorKind as G;
        match e {
            F::Underflow(..) => G::Underflow,
            F::Overflow(..) => G::Overflow,
            F::NotANumber(..) => G::Unrepresentable,
        }
    }
}

/**
Indicates that it is not possible for the conversion to fail.

You can use the [`UnwrapOk::unwrap_ok`](./trait.UnwrapOk.html#tymethod.unwrap_ok) method to discard the (statically impossible) `Err` case from a `Result<_, NoError>`, without using `Result::unwrap` (which is typically viewed as a "code smell").
*/
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum NoError {}

impl Display for NoError {
    fn fmt(&self, _: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        unreachable!()
    }
}

impl Error for NoError {
    fn description(&self) -> &str {
        unreachable!()
    }
}

custom_derive! {
    /// Indicates that the conversion failed because the value was not representable.
    #[derive(
        Copy, Clone, Eq, PartialEq, Ord, PartialOrd,
        IntoInner, DummyDebug, FromNoError,
        Desc("could not convert unrepresentable value")
    )]
    pub struct Unrepresentable<T>(pub T);
}

custom_derive! {
    /// Indicates that the conversion failed due to an underflow.
    #[derive(
        Copy, Clone, Eq, PartialEq, Ord, PartialOrd,
        IntoInner, DummyDebug, FromNoError,
        Desc("conversion resulted in underflow")
    )]
    pub struct Underflow<T>(pub T);
}

custom_derive! {
    /// Indicates that the conversion failed due to an overflow.
    #[derive(
        Copy, Clone, Eq, PartialEq, Ord, PartialOrd,
        IntoInner, DummyDebug, FromNoError,
        Desc("conversion resulted in overflow")
    )]
    pub struct Overflow<T>(pub T);
}

custom_derive! {
    /**
    Indicates that a conversion from a floating point type failed.
    */
    #[derive(
        Copy, Clone, Eq, PartialEq, Ord, PartialOrd,
        IntoInner, DummyDebug, FromNoError,
        EnumDesc(
            Underflow => "conversion resulted in underflow",
            Overflow => "conversion resulted in overflow",
            NotANumber => "conversion target does not support not-a-number",
        ),
        FromName(Underflow),
        FromName(Overflow),
        FromRemap(RangeError(Underflow, Overflow))
    )]
    pub enum FloatError<T> {
        /// Input underflowed the target type.
        Underflow(T),

        /// Input overflowed the target type.
        Overflow(T),

        /// Input was not-a-number, which the target type could not represent.
        NotANumber(T),
    }
}

custom_derive! {
    /**
    Indicates that a conversion failed due to a range error.
    */
    #[derive(
        Copy, Clone, Eq, PartialEq, Ord, PartialOrd,
        IntoInner, DummyDebug, FromNoError,
        EnumDesc(
            Underflow => "conversion resulted in underflow",
            Overflow => "conversion resulted in overflow",
        ),
        FromName(Underflow),
        FromName(Overflow)
    )]
    pub enum RangeError<T> {
        /// Input underflowed the target type.
        Underflow(T),

        /// Input overflowed the target type.
        Overflow(T),
    }
}

custom_derive! {
    /**
    Indicates that a conversion failed due to a range error.

    This is a variant of `RangeError` that does not retain the input value which caused the error.  It exists to help unify some utility methods and should not generally be used directly, unless you are targeting the `Unwrap*` traits.
    */
    #[derive(
        Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug,
        FromNoError,
        EnumDesc(
            Underflow => "conversion resulted in underflow",
            Overflow => "conversion resulted in overflow",
        ),
        FromName(Underflow<T>),
        FromName(Overflow<T>),
        FromRemap(RangeError<T>(Underflow, Overflow))
    )]
    pub enum RangeErrorKind {
        /// Input underflowed the target type.
        Underflow,

        /// Input overflowed the target type.
        Overflow,
    }
}

/**
Saturates a `Result`.
*/
pub trait Saturate {
    /// The result of saturating.
    type Output;

    /**
    Replaces an overflow or underflow error with a saturated value.

    Unlike `unwrap_or_saturate`, this method can be used in cases where the `Result` error type can encode failures *other* than overflow and underflow.  For example, you cannot saturate a float-to-integer conversion using `unwrap_or_saturate` as the error might be `NotANumber`, which doesn't have a meaningful saturation "direction".

    The output of this method will be a `Result` where the error type *does not* contain overflow or underflow conditions.  What conditions remain must still be dealt with in some fashion.
    */
    fn saturate(self) -> Self::Output;
}

impl<T, U> Saturate for Result<T, FloatError<U>>
where T: Saturated {
    type Output = Result<T, Unrepresentable<U>>;

    #[inline]
    fn saturate(self) -> Self::Output {
        use self::FloatError::*;
        match self {
            Ok(v) => Ok(v),
            Err(Underflow(_)) => Ok(T::saturated_min()),
            Err(Overflow(_)) => Ok(T::saturated_max()),
            Err(NotANumber(v)) => Err(Unrepresentable(v))
        }
    }
}

impl<T, U> Saturate for Result<T, RangeError<U>>
where T: Saturated {
    type Output = Result<T, NoError>;

    #[inline]
    fn saturate(self) -> Self::Output {
        use self::RangeError::*;
        match self {
            Ok(v) => Ok(v),
            Err(Underflow(_)) => Ok(T::saturated_min()),
            Err(Overflow(_)) => Ok(T::saturated_max())
        }
    }
}

impl<T> Saturate for Result<T, RangeErrorKind>
where T: Saturated {
    type Output = Result<T, NoError>;

    #[inline]
    fn saturate(self) -> Self::Output {
        use self::RangeErrorKind::*;
        match self {
            Ok(v) => Ok(v),
            Err(Underflow) => Ok(T::saturated_min()),
            Err(Overflow) => Ok(T::saturated_max())
        }
    }
}

/**
Safely unwrap a `Result` that cannot contain an error.
*/
pub trait UnwrapOk<T> {
    /**
    Unwraps a `Result` without possibility of failing.

    Technically, this is not necessary; it's provided simply to make user code a little clearer.
    */
    fn unwrap_ok(self) -> T;
}

impl<T> UnwrapOk<T> for Result<T, NoError> {
    #[inline]
    fn unwrap_ok(self) -> T {
        match self {
            Ok(v) => v,
            Err(..) => loop {},
        }
    }
}

/**
Unwrap a conversion by saturating to infinity.
*/
pub trait UnwrapOrInf {
    /// The result of unwrapping.
    type Output;

    /**
    Either unwraps the successfully converted value, or saturates to infinity in the "direction" of overflow/underflow.
    */
    fn unwrap_or_inf(self) -> Self::Output;
}

/**
Unwrap a conversion by replacing a failure with an invalid sentinel value.
*/
pub trait UnwrapOrInvalid {
    /// The result of unwrapping.
    type Output;

    /**
    Either unwraps the successfully converted value, or returns the output type's invalid sentinel value.
    */
    fn unwrap_or_invalid(self) -> Self::Output;
}

/**
Unwrap a conversion by saturating.
*/
pub trait UnwrapOrSaturate {
    /// The result of unwrapping.
    type Output;

    /**
    Either unwraps the successfully converted value, or saturates in the "direction" of overflow/underflow.
    */
    fn unwrap_or_saturate(self) -> Self::Output;
}

impl<T, E> UnwrapOrInf for Result<T, E>
where T: SignedInfinity, E: Into<RangeErrorKind> {
    type Output = T;
    #[inline]
    fn unwrap_or_inf(self) -> T {
        use self::RangeErrorKind::*;
        match self.map_err(Into::into) {
            Ok(v) => v,
            Err(Underflow(..)) => T::neg_infinity(),
            Err(Overflow(..)) => T::pos_infinity(),
        }
    }
}

impl<T, E> UnwrapOrInvalid for Result<T, E>
where T: InvalidSentinel {
    type Output = T;
    #[inline]
    fn unwrap_or_invalid(self) -> T {
        match self {
            Ok(v) => v,
            Err(..) => T::invalid_sentinel(),
        }
    }
}

impl<T, E> UnwrapOrSaturate for Result<T, E>
where T: Saturated, E: Into<RangeErrorKind> {
    type Output = T;
    #[inline]
    fn unwrap_or_saturate(self) -> T {
        use self::RangeErrorKind::*;
        match self.map_err(Into::into) {
            Ok(v) => v,
            Err(Underflow(..)) => T::saturated_min(),
            Err(Overflow(..)) => T::saturated_max(),
        }
    }
}
