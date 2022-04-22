#[macro_export]
macro_rules! make_build_info {
    () => {
        use std::{collections, env, path};
        let src = env::var("CARGO_MANIFEST_DIR").unwrap();
        let dst = path::Path::new(&env::var("OUT_DIR").unwrap()).join("built.rs");
        let mut option = built::Options::default();
        option.set_dependencies(true);
        built::write_built_file_with_opts(&option, src.as_ref(), &dst)
            .expect("Failed to acquire build-time information");
    };
}

#[macro_export]
macro_rules! make_decimal {
    () => {
        use once_cell::sync::Lazy;
        use pyo3::class::basic::CompareOp;
        use pyo3::conversion::AsPyPointer;
        use pyo3::prelude::*;
        use pyo3::PyNativeType;
        use pyo3::{exceptions, PyResult};
        use std::fmt::Display;
        use std::ops::{Deref, DerefMut};

        use rust_decimal::prelude::Decimal as RustDecimal;
        use rust_decimal::prelude::ToPrimitive;
        use std::fmt;
        use std::ptr;

        use fxhash::hash;
        use std::hash::Hash;
        #[derive(Hash, Debug, Clone)]
        struct VersionInfo {
            rustc: String,
            os: String,
            family: String,
            env: String,
            endian: String,
            arch: String,
            target: String,
            rust_decimal: String,
            pyo3: String,
        }

        pub mod built_info {
            include!(concat!(env!("OUT_DIR"), "/built.rs"));
        }

        fn make_decimal_version_info() -> VersionInfo {
            let rustc = built_info::RUSTC_VERSION.to_string();
            let os = built_info::CFG_OS.to_string();
            let family = built_info::CFG_FAMILY.to_string();
            let env = built_info::CFG_ENV.to_string();
            let endian = built_info::CFG_ENDIAN.to_string();
            let arch = built_info::CFG_TARGET_ARCH.to_string();
            let target = built_info::TARGET.to_string();
            let rust_decimal = built_info::DEPENDENCIES
                .iter()
                .filter(|&dep| dep.0 == "rust_decimal")
                .map(|&dep| dep.1)
                .next()
                .expect("dependency of rust_decimal was not found")
                .to_string();
            let pyo3 = built_info::DEPENDENCIES
                .iter()
                .filter(|&dep| dep.0 == "pyo3")
                .map(|&dep| dep.1)
                .next()
                .expect("dependency of rust_decimal was not found")
                .to_string();
            let ver_info = VersionInfo {
                rustc,
                os,
                family,
                env,
                endian,
                arch,
                target,
                rust_decimal,
                pyo3,
            };
            ver_info
        }
        fn make_decimal_version_hash() -> usize {
            let ver_info = make_decimal_version_info();
            hash(&ver_info)
        }
        static DECIMAL_VERSION_HASH: Lazy<usize> = Lazy::new(|| make_decimal_version_hash());
        static DECIMAL_VERSION_INFO: Lazy<VersionInfo> = Lazy::new(|| make_decimal_version_info());

        #[pyfunction]
        fn get_decimal_version_info<'p>(input: Decimal, py: Python<'p>) -> PyResult<String> {
            Ok(format!("{:?}", *DECIMAL_VERSION_INFO).to_string())
        }

        #[pyclass(module = "pyo3_decimal", name = "Decimal")]
        #[derive(Debug)]
        pub struct Decimal(RustDecimal, usize);
        pub struct Wrapper(PyCell<Decimal>);
        unsafe impl PyNativeType for Wrapper {}

        impl<'source> FromPyObject<'source> for Decimal {
            fn extract(ob: &'source PyAny) -> PyResult<Self> {
                let _cell = unsafe { Wrapper::unchecked_downcast(ob) };
                let unwrapped: &Decimal = &_cell.0.try_borrow().unwrap();
                if *DECIMAL_VERSION_HASH != unwrapped.1 {
                    return Err(exceptions::PyValueError::new_err(format!(
                        "VERSION HASH not the same. {:?}",
                        *DECIMAL_VERSION_INFO
                    )));
                }
                Ok(Decimal(unwrapped.0.clone(), *DECIMAL_VERSION_HASH))
            }
        }
        impl fmt::Display for Decimal {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        #[pymethods]
        impl Decimal {
            #[new]
            pub fn new(num: i128, scale: u32) -> Decimal {
                Self(
                    RustDecimal::from_i128_with_scale(num, scale),
                    *DECIMAL_VERSION_HASH,
                )
            }

            pub const fn scale(&self) -> u32 {
                self.0.scale()
            }

            pub const fn mantissa(&self) -> i128 {
                self.0.mantissa()
            }

            pub const fn is_zero(&self) -> bool {
                self.0.is_zero()
            }

            pub fn set_sign_positive(&mut self, positive: bool) {
                self.0.set_sign_positive(positive)
            }

            //#[inline(always)]
            pub fn set_sign_negative(&mut self, negative: bool) {
                self.0.set_sign_negative(negative)
            }

            pub fn set_scale(&mut self, scale: u32) -> PyResult<()> {
                let result = self.0.set_scale(scale);
                match result {
                    Ok(v) => Ok(v),
                    Err(_) => Err(exceptions::PyRuntimeError::new_err("set_scale Error")),
                }
            }

            pub fn rescale(&mut self, scale: u32) {
                self.0.rescale(scale)
            }

            pub const fn is_sign_negative(&self) -> bool {
                self.0.is_sign_negative()
            }

            pub const fn is_sign_positive(&self) -> bool {
                self.0.is_sign_positive()
            }

            pub fn trunc(&self) -> Decimal {
                self.0.trunc().into()
            }

            pub fn fract(&self) -> Decimal {
                self.0.fract().into()
            }

            pub fn abs(&self) -> Decimal {
                self.0.abs().into()
            }

            pub fn floor(&self) -> Decimal {
                self.0.floor().into()
            }

            pub fn ceil(&self) -> Decimal {
                self.0.ceil().into()
            }

            pub fn max(&self, other: Decimal) -> Decimal {
                self.0.max(other.0).into()
            }

            pub fn min(&self, other: Decimal) -> Decimal {
                self.0.min(other.0).into()
            }

            pub fn normalize(&self) -> Decimal {
                self.0.normalize().into()
            }

            pub fn normalize_assign(&mut self) {
                self.0.normalize_assign()
            }

            pub fn round(&self) -> Decimal {
                self.0.round().into()
            }

            pub fn round_dp(&self, dp: u32) -> Decimal {
                self.0.round_dp(dp).into()
            }

            pub fn round_sf(&self, digits: u32) -> Option<Decimal> {
                let decimal = self.0.round_sf(digits);
                if decimal.is_some() {
                    Some(decimal.unwrap().into())
                } else {
                    None
                }
            }

            pub fn to_int(&self) -> i64 {
                self.0.to_i64().unwrap()
            }

            pub fn to_float(&self) -> f64 {
                self.0.to_f64().unwrap()
            }

            fn __add__(&self, other: Decimal) -> PyResult<Decimal> {
                Ok((self.0 + other.0).into())
            }

            fn __sub__(&self, other: Decimal) -> PyResult<Decimal> {
                Ok((self.0 - other.0).into())
            }

            fn __mult__(&self, other: Decimal) -> PyResult<Decimal> {
                Ok((self.0 * other.0).into())
            }

            fn __mod__(&self, other: Decimal) -> PyResult<Decimal> {
                Ok((self.0 / other.0).into())
            }

            fn __divmod__(&self, other: Decimal) -> PyResult<Decimal> {
                Ok((self.0 / other.0).into())
            }

            fn __richcmp__(&self, other: Decimal, op: CompareOp) -> PyResult<bool> {
                match op {
                    CompareOp::Lt => Ok(self.0 < other.0),
                    CompareOp::Le => Ok(self.0 <= other.0),
                    CompareOp::Eq => Ok(self.0 == other.0),
                    CompareOp::Ne => Ok(self.0 != other.0),
                    CompareOp::Gt => Ok(self.0 > other.0),
                    CompareOp::Ge => Ok(self.0 >= other.0),
                }
            }

            fn __str__(&self) -> PyResult<String> {
                Ok(self.to_string())
            }

            fn __repr__(&self) -> PyResult<String> {
                Ok(format!("Decimal({})", self.to_string()))
            }
        }

        impl Deref for Decimal {
            type Target = RustDecimal;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl DerefMut for Decimal {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl Into<RustDecimal> for Decimal {
            fn into(self) -> RustDecimal {
                self.0
            }
        }

        impl Into<Decimal> for RustDecimal {
            fn into(self) -> Decimal {
                Decimal(self, *DECIMAL_VERSION_HASH)
            }
        }
    };
}
