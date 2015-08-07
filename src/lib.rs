/*!
This crate provides a number of conversion traits with more specific semantics than those provided by `as` or `From`/`Into`.

The goal with the traits provided here is to be more specific about what generic code can rely on, as well as provide reasonably self-describing alternatives to the standard `From`/`Into` traits.  For example, the although `T: From<U>` might be satisfied in generic code, this says nothing about what *kind* of conversion that represents.

In addition, `From`/`Into` provide no facility for a conversion failing, meaning that implementations may need to choose between conversions that may not be valid, or panicking; neither option is appealing in general.

# API Stability Notice

The API of this crate is still not entirely decided.  In particular, errors may change in the future to carry the value that failed to convert (allowing it to be recovered).

# Overview

The following traits are used to define various conversion semantics:

- [`ApproxFrom`](./trait.ApproxFrom.html)/[`ApproxInto`](./trait.ApproxInto.html) - approximate conversions, with selectable approximation scheme (see [`ApproxScheme`](./trait.ApproxScheme.html)).
- [`TryFrom`](./trait.TryFrom.html)/[`TryInto`](./trait.TryInto.html) - general, potentially failing value conversions.
- [`ValueFrom`](./trait.ValueFrom.html)/[`ValueInto`](./trait.ValueInto.html) - exact, value-preserving conversions.

These extension methods are used to make working with potentially failing conversions simpler:

- [`UnwrapOk::unwrap_ok`](./errors/trait.UnwrapOk.html#tymethod.unwrap_ok) - unwraps results from conversions that cannot fail.
- [`UnwrapOrInf::unwrap_or_inf`](./errors/trait.UnwrapOrInf.html#tymethod.unwrap_or_inf) - saturates to ±∞ on failure.
- [`UnwrapOrInvalid::unwrap_or_invalid`](./errors/trait.UnwrapOrInvalid.html#tymethod.unwrap_or_invalid) - substitutes the target type's "invalid" sentinel value on failure.
- [`UnwrapOrSaturate::unwrap_or_saturate`](./errors/trait.UnwrapOrSaturate.html#tymethod.unwrap_or_saturate) - saturates to the maximum or minimum value of the target type on failure.

A macro is provided to assist in implementing conversions:

- [`TryFrom!`](./macros/index.html#tryfrom!) - derives an implementation of [`TryFrom`](./trait.TryFrom.html).

If you are implementing your own types, you may also be interested in the traits contained in the [`misc`](./misc/index.html) module.
*/

#![deny(missing_docs)]

// Exported macros.
pub mod macros;

pub use errors::{
    NoError, GeneralError, Unrepresentable,
    Underflow, Overflow,
    FloatError, RangeError,
    UnwrapOk, UnwrapOrInf, UnwrapOrInvalid, UnwrapOrSaturate,
};

/**
Publicly re-exports the most generally useful set of items.

Usage of the prelude should be considered **unstable**.  Although items will likely *not* be removed without bumping the major version, new items *may* be added, which could potentially cause name conflicts in user code.
*/
pub mod prelude {
    pub use super::{
        ApproxFrom, ApproxInto, ApproxWith,
        ValueFrom, ValueInto,
        UnwrapOk, UnwrapOrInf, UnwrapOrInvalid, UnwrapOrSaturate,
    };
}

macro_rules! as_item {
    ($($i:item)*) => {$($i)*};
}

macro_rules! item_for_each {
    (
        $( ($($arg:tt)*) ),* $(,)* => { $($exp:tt)* }
    ) => {
        macro_rules! body {
            $($exp)*
        }

        $(
            body! { $($arg)* }
        )*
    };
}

pub mod errors;
pub mod misc;

mod impls;

/**
This trait is used to perform a conversion that is permitted to approximate the result, but *not* to wrap or saturate the result to fit into the destination type's representable range.

# Details

All implementations of this trait must provide a conversion that can be separated into two logical steps: an approximation transform, and a representation transform.

The "approximation transform" step involves transforming the input value into an approximately equivalent value which is supported by the target type *without* taking the target type's representable range into account.  For example, this might involve rounding or truncating a floating point value to an integer, or reducing the accuracy of a floating point value.

The "representation transform" step *exactly* rewrites the value from the source type's binary representation into the destination type's binary representation.  This step *may not* transform the value in any way.  If the result of the approximation is not representable, the conversion *must* fail.

The major reason for this formulation is to exactly define what happens when converting between floating point and integer types.  Often, it is unclear what happens to floating point values beyond the range of the target integer type.  Do they saturate, wrap, or cause a failure?

With this formulation, it is well-defined: if a floating point value is outside the representable range, the conversion fails.  This allows users to distinguish between approximation and range violation, and act accordingly.
*/
pub trait ApproxFrom<Src, Scheme=DefaultApprox> where Scheme: ApproxScheme {
    /// The error type produced by a failed conversion.
    type Err;

    /// Convert the given value into an approximately equivalent representation.
    fn approx_from(src: Src) -> Result<Self, Self::Err>;
}

impl<Src, Scheme> ApproxFrom<Src, Scheme> for Src where Scheme: ApproxScheme {
    type Err = NoError;
    fn approx_from(src: Src) -> Result<Self, Self::Err> {
        Ok(src)
    }
}

/**
This is the dual of `ApproxFrom`; see that trait for information.
*/
pub trait ApproxInto<Dst, Scheme=DefaultApprox> where Scheme: ApproxScheme {
    /// The error type produced by a failed conversion.
    type Err;

    /// Convert the subject into an approximately equivalent representation.
    fn approx_into(self) -> Result<Dst, Self::Err>;
}

impl<Dst, Src, Scheme> ApproxInto<Dst, Scheme> for Src
where
    Dst: ApproxFrom<Src, Scheme>,
    Scheme: ApproxScheme,
{
    type Err = Dst::Err;
    fn approx_into(self) -> Result<Dst, Self::Err> {
        ApproxFrom::approx_from(self)
    }
}

/**
This extension trait exists to simplify using approximation implementations.

If there is more than one `ApproxFrom` implementation for a given type, a simple call to `approx_into` may not be uniquely resolvable.  Due to the position of the scheme parameter (on the trait itself), it is cumbersome to specify which scheme you wanted.

Hence this trait.
*/
pub trait ApproxWith<Dst> {
    /// Approximate the subject with the default scheme.
    fn approx(self) -> Result<Dst, Self::Err>
    where Self: Sized + ApproxInto<Dst> {
        self.approx_into()
    }

    /// Approximate the subject with a specific scheme.
    fn approx_with<Scheme=DefaultApprox>(self) -> Result<Dst, Self::Err>
    where
        Self: Sized + ApproxInto<Dst, Scheme>,
        Scheme: ApproxScheme,
    {
        self.approx_into()
    }
}

impl<T, Dst> ApproxWith<Dst> for T {}

/**
This trait is used to mark approximation scheme types.
*/
pub trait ApproxScheme {}

/**
The "default" approximation scheme.  This scheme does whatever would generally be expected of a lossy conversion, assuming no additional context or instruction is given.

This is a double-edged sword: it has the loosest semantics, but is far more likely to exist than more complicated approximation schemes.
*/
pub enum DefaultApprox {}
impl ApproxScheme for DefaultApprox {}

/**
This scheme is used to convert a value by "wrapping" it into a narrower range.

In abstract, this can be viewed as the opposite of rounding: rather than preserving the most significant bits of a value, it preserves the *least* significant bits of a value.
*/
pub enum Wrapping {}
impl ApproxScheme for Wrapping {}

// TODO: RoundToNearest, RoundToPosInf, RoundToNegInf, RoundToZero

/**
This trait is used to perform a conversion between different semantic types which might fail.

# Details

Typically, this should be used in cases where you are converting between values whose ranges and/or representations only partially overlap.  That the conversion may fail should be a reasonably expected outcome.  A standard example of this is converting from integers to enums of unitary variants.
*/
pub trait TryFrom<Src> {
    /// The error type produced by a failed conversion.
    type Err;

    /// Convert the given value into the subject type.
    fn try_from(src: Src) -> Result<Self, Self::Err>;
}

impl<Src> TryFrom<Src> for Src {
    type Err = NoError;
    fn try_from(src: Src) -> Result<Self, Self::Err> {
        Ok(src)
    }
}

/**
This is the dual of `TryFrom`; see that trait for information.
*/
pub trait TryInto<Dst> {
    /// The error type produced by a failed conversion.
    type Err;

    /// Convert the subject into the destination type.
    fn try_into(self) -> Result<Dst, Self::Err>;
}

impl<Src, Dst> TryInto<Dst> for Src where Dst: TryFrom<Src> {
    type Err = Dst::Err;
    fn try_into(self) -> Result<Dst, Self::Err> {
        TryFrom::try_from(self)
    }
}

/**
This trait is used to perform an exact, value-preserving conversion.

# Details

Implementations of this trait should be reflexive, associative and commutative (in the absence of conversion errors).  That is, all possible cycles of `ValueFrom` conversions (for which each "step" has a defined implementation) should produce the same result, with a given value either being "round-tripped" exactly, or an error being produced.
*/
pub trait ValueFrom<Src> {
    /// The error type produced by a failed conversion.
    type Err;

    /// Convert the given value into an exactly equivalent representation.
    fn value_from(src: Src) -> Result<Self, Self::Err>;
}

impl<Src> ValueFrom<Src> for Src {
    type Err = NoError;
    fn value_from(src: Src) -> Result<Self, Self::Err> {
        Ok(src)
    }
}

/**
This is the dual of `ValueFrom`; see that trait for information.
*/
pub trait ValueInto<Dst> {
    /// The error type produced by a failed conversion.
    type Err;
    
    /// Convert the subject into an exactly equivalent representation.
    fn value_into(self) -> Result<Dst, Self::Err>;
}

impl<Src, Dst> ValueInto<Dst> for Src where Dst: ValueFrom<Src> {
    type Err = Dst::Err;
    fn value_into(self) -> Result<Dst, Self::Err> {
        ValueFrom::value_from(self)
    }
}