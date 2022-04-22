# PyO3 wrapper for rust_decimal

## Purpose

I tried to use python decimal C-API because it'd be more convinient. But Python's decimal is under discussion and it will take time to implement C-API. Decimal C interface was about to release on Python version 3.10 but it was reverted.

That made me decide not to use python's native decimal any more and use this library.

This crate provides you the subset of macros to implement Decimal to Python and read from rust side as well. It needs same development environment such as versions of `pyo3` and `rust_decimal` because it uses manual memory parsing with using `pyo3::PyNativeType::unchecked_downcast`. On memory Parsing, check if the environment parameters hash of the library is the same.
