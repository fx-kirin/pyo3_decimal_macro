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
        use pyo3::PyNativeType; // これは trait なので消せない
        use rust_decimal::prelude::FromStr;
        use rust_decimal::prelude::ToPrimitive;

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
            fxhash::hash(&ver_info)
        }
        static DECIMAL_VERSION_HASH: once_cell::sync::Lazy<usize> =
            once_cell::sync::Lazy::new(|| make_decimal_version_hash());
        static DECIMAL_VERSION_INFO: once_cell::sync::Lazy<VersionInfo> =
            once_cell::sync::Lazy::new(|| make_decimal_version_info());

        #[pyfunction]
        fn get_decimal_version_info<'p>(input: Decimal, _py: Python<'p>) -> pyo3::PyResult<String> {
            Ok(format!("{:?}", *DECIMAL_VERSION_INFO).to_string())
        }

        #[pyclass(module = "pyo3_decimal", name = "Decimal")]
        #[derive(Debug)]
        #[repr(C)]
        pub struct Decimal(rust_decimal::prelude::Decimal, usize);
        pub struct Wrapper(PyCell<Decimal>);
        unsafe impl pyo3::PyNativeType for Wrapper {}

        impl<'source> FromPyObject<'source> for Decimal {
            fn extract(ob: &'source pyo3::types::PyAny) -> pyo3::PyResult<Self> {
                let py_int = ob.cast_as::<pyo3::types::PyInt>();

                if let Ok(content) = py_int {
                    let num: i128 = content.extract().unwrap();
                    return Ok(Decimal(
                        rust_decimal::prelude::Decimal::from_i128_with_scale(num, 0),
                        *DECIMAL_VERSION_HASH,
                    ));
                }
                let _cell = unsafe { Wrapper::unchecked_downcast(ob) };
                let unwrapped: &Decimal = &_cell.0.try_borrow().unwrap();
                if *DECIMAL_VERSION_HASH != unwrapped.1 {
                    return Err(pyo3::PyDowncastError::new(
                        ob,
                        format!(
                            "Decimal. Input error. VERSION HASH is not the same. {:?}",
                            *DECIMAL_VERSION_INFO
                        ),
                    )
                    .into());
                }
                Ok(Decimal(unwrapped.0, *DECIMAL_VERSION_HASH))
            }
        }

        impl std::fmt::Display for Decimal {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        #[pymethods]
        impl Decimal {
            #[new]
            #[args(arg1, arg2 = "None")]
            pub fn new<'p>(
                arg1: PyObject,
                arg2: Option<PyObject>,
                py: Python<'p>,
            ) -> pyo3::PyResult<Decimal> {
                let py_string = arg1.cast_as::<pyo3::types::PyString>(py);
                if let Ok(content) = py_string {
                    let rust_str: &str = &content.to_str().unwrap();
                    let result = rust_decimal::Decimal::from_str(rust_str);
                    if arg2.is_some() {
                        return Err(pyo3::exceptions::PyValueError::new_err(format!(
                            "arg1 is String but arg2 was supplied value. {:?}",
                            arg2
                        )));
                    }
                    return match result {
                        Ok(v) => Ok(Self(v, *DECIMAL_VERSION_HASH)),
                        Err(_) => Err(pyo3::exceptions::PyValueError::new_err(format!(
                            "arg1 is wrong value. {}",
                            rust_str
                        ))),
                    };
                }
                let py_float = arg1.cast_as::<pyo3::types::PyFloat>(py);
                if let Ok(content) = py_float {
                    let num: f64 = content.extract().unwrap();
                    if arg2.is_some() {
                        return Err(pyo3::exceptions::PyValueError::new_err(format!(
                            "arg1 is Float but arg2 was supplied value. {:?}",
                            arg2
                        )));
                    }
                    return Ok(Self(
                        rust_decimal::prelude::Decimal::from_f64_retain(num)
                            .expect("Failed to load from float value"),
                        *DECIMAL_VERSION_HASH,
                    ));
                }
                let py_int = arg1.cast_as::<pyo3::types::PyInt>(py);
                let num: i128 = if let Ok(content) = py_int {
                    content.extract().unwrap()
                } else {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "arg1 is wrong value. {:?}",
                        arg1
                    )));
                };
                let scale: u32 = if let Some(arg2) = arg2 {
                    let py_int = arg2.cast_as::<pyo3::types::PyInt>(py);
                    if let Ok(content) = py_int {
                        content.extract().unwrap()
                    } else {
                        return Err(pyo3::exceptions::PyValueError::new_err(format!(
                            "arg2 is wrong value. {:?}",
                            arg2
                        )));
                    }
                } else {
                    0
                };
                Ok(Self(
                    rust_decimal::prelude::Decimal::from_i128_with_scale(num, scale),
                    *DECIMAL_VERSION_HASH,
                ))
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

            pub fn set_scale(&mut self, scale: u32) -> pyo3::PyResult<()> {
                let result = self.0.set_scale(scale);
                match result {
                    Ok(v) => Ok(v),
                    Err(_) => Err(pyo3::exceptions::PyRuntimeError::new_err("set_scale Error")),
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

            fn __add__(&self, other: &Decimal) -> pyo3::PyResult<Decimal> {
                Ok((self.0 + other.0).into())
            }

            fn __sub__(&self, other: &Decimal) -> pyo3::PyResult<Decimal> {
                Ok((self.0 - other.0).into())
            }

            fn __mul__(&self, other: &Decimal) -> pyo3::PyResult<Decimal> {
                Ok((self.0 * other.0).into())
            }

            fn __truediv__(&self, other: &Decimal) -> pyo3::PyResult<Decimal> {
                Ok((self.0 / other.0).into())
            }

            fn __floordiv__(&self, other: &Decimal) -> pyo3::PyResult<Decimal> {
                Ok((self.0 / other.0).into())
            }

            fn __neg__(&self) -> pyo3::PyResult<Decimal> {
                Ok((-self.0).into())
            }

            fn __richcmp__(
                &self,
                other: Decimal,
                op: pyo3::class::basic::CompareOp,
            ) -> pyo3::PyResult<bool> {
                match op {
                    pyo3::class::basic::CompareOp::Lt => Ok(self.0 < other.0),
                    pyo3::class::basic::CompareOp::Le => Ok(self.0 <= other.0),
                    pyo3::class::basic::CompareOp::Eq => Ok(self.0 == other.0),
                    pyo3::class::basic::CompareOp::Ne => Ok(self.0 != other.0),
                    pyo3::class::basic::CompareOp::Gt => Ok(self.0 > other.0),
                    pyo3::class::basic::CompareOp::Ge => Ok(self.0 >= other.0),
                }
            }

            fn __str__(&self) -> pyo3::PyResult<String> {
                Ok(self.to_string())
            }

            fn __repr__(&self) -> pyo3::PyResult<String> {
                Ok(format!("Decimal({})", self.to_string()))
            }

            fn __int__(&self) -> i64 {
                self.to_int()
            }

            fn __float__(&self) -> f64 {
                self.to_float()
            }

            fn __format__(&self, format_spec: &str) -> pyo3::PyResult<String> {
                let text_length = format_spec.len();
                if text_length == 0 {
                    return Ok(self.to_string());
                }
                let format_base = &format_spec[text_length - 1..text_length];
                if format_base == "i" {
                    if text_length == 1 {
                        return Ok(self.to_int().to_string());
                    }
                    let format_prefix = &format_spec[0..(text_length - 1)];
                    let result = num_runtime_fmt::NumFmt::from_str(&*format_prefix);
                    let result = match result {
                        Ok(r) => r,
                        Err(e) => {
                            return Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                                "format string error {}",
                                e.to_string()
                            )));
                        }
                    };
                    let result = result.fmt(self.to_int());
                    let result = match result {
                        Ok(r) => r,
                        Err(e) => {
                            return Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                                "format string error {}",
                                e.to_string()
                            )));
                        }
                    };
                    return Ok(result);
                } else if format_base == "f" {
                    if text_length == 1 {
                        return Ok(self.to_float().to_string());
                    }
                    let format_prefix = &format_spec[0..(text_length - 1)];
                    let result = num_runtime_fmt::NumFmt::from_str(&*format_prefix);
                    let result = match result {
                        Ok(r) => r,
                        Err(e) => {
                            return Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                                "format string error {}",
                                e.to_string()
                            )));
                        }
                    };
                    let result = result.fmt(self.to_float());
                    let result = match result {
                        Ok(r) => r,
                        Err(e) => {
                            return Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                                "format string error {}",
                                e.to_string()
                            )));
                        }
                    };
                    return Ok(result);
                } else {
                    let result = num_runtime_fmt::NumFmt::from_str(&*format_spec);
                    let result = match result {
                        Ok(r) => r,
                        Err(e) => {
                            return Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                                "format string error {}",
                                e.to_string()
                            )));
                        }
                    };
                    let result = result.fmt(self.to_float());
                    let result = match result {
                        Ok(r) => r,
                        Err(e) => {
                            return Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                                "format string error {}",
                                e.to_string()
                            )));
                        }
                    };
                    return Ok(result);
                }
            }
        }
        impl Decimal {
            pub fn from_i128_with_scale<'p>(num: i128, scale: u32) -> Decimal {
                Self(
                    rust_decimal::prelude::Decimal::from_i128_with_scale(num, 0),
                    *DECIMAL_VERSION_HASH,
                )
            }
        }

        impl std::ops::Deref for Decimal {
            type Target = rust_decimal::prelude::Decimal;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::ops::DerefMut for Decimal {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl Into<rust_decimal::prelude::Decimal> for Decimal {
            fn into(self) -> rust_decimal::prelude::Decimal {
                self.0
            }
        }

        impl Into<Decimal> for rust_decimal::prelude::Decimal {
            fn into(self) -> Decimal {
                Decimal(self, *DECIMAL_VERSION_HASH)
            }
        }
    };
}

