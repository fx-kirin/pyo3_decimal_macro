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
        use pyo3::types::{PyAny, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple};
        use pyo3::PyNativeType;
        use pyo3::{exceptions, PyResult};
        use std::fmt::Display;
        use std::ops::{Deref, DerefMut};
        use std::str::FromStr;

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
        fn get_decimal_version_info<'p>(input: Decimal, _py: Python<'p>) -> PyResult<String> {
            Ok(format!("{:?}", *DECIMAL_VERSION_INFO).to_string())
        }

        #[pyclass(module = "pyo3_decimal", name = "Decimal")]
        #[derive(Debug)]
        #[repr(C)]
        pub struct Decimal(RustDecimal, usize);
        pub struct Wrapper(PyCell<Decimal>);
        unsafe impl PyNativeType for Wrapper {}

        impl<'source> FromPyObject<'source> for Decimal {
            fn extract(ob: &'source PyAny) -> PyResult<Self> {
                let py_int = ob.cast_as::<PyInt>();

                if let Ok(content) = py_int {
                    let num: i128 = content.extract().unwrap();
                    return Ok(Decimal(
                        RustDecimal::from_i128_with_scale(num, 0),
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

        impl fmt::Display for Decimal {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
            ) -> PyResult<Decimal> {
                let py_string = arg1.cast_as::<PyString>(py);
                if let Ok(content) = py_string {
                    let rust_str: &str = &content.to_str().unwrap();
                    let result = RustDecimal::from_str(rust_str);
                    if arg2.is_some() {
                        return Err(exceptions::PyValueError::new_err(format!(
                            "arg1 is String but arg2 was supplied value. {:?}",
                            arg2
                        )));
                    }
                    return match result {
                        Ok(v) => Ok(Self(v, *DECIMAL_VERSION_HASH)),
                        Err(_) => Err(exceptions::PyValueError::new_err(format!(
                            "arg1 is wrong value. {}",
                            rust_str
                        ))),
                    };
                }
                let py_float = arg1.cast_as::<PyFloat>(py);
                if let Ok(content) = py_float {
                    let num: f64 = content.extract().unwrap();
                    if arg2.is_some() {
                        return Err(exceptions::PyValueError::new_err(format!(
                            "arg1 is Float but arg2 was supplied value. {:?}",
                            arg2
                        )));
                    }
                    return Ok(Self(
                        RustDecimal::from_f64_retain(num).expect("Failed to load from float value"),
                        *DECIMAL_VERSION_HASH,
                    ));
                }
                let py_int = arg1.cast_as::<PyInt>(py);
                let num: i128 = if let Ok(content) = py_int {
                    content.extract().unwrap()
                } else {
                    return Err(exceptions::PyValueError::new_err(format!(
                        "arg1 is wrong value. {:?}",
                        arg1
                    )));
                };
                let scale: u32 = if let Some(arg2) = arg2 {
                    let py_int = arg2.cast_as::<PyInt>(py);
                    if let Ok(content) = py_int {
                        content.extract().unwrap()
                    } else {
                        return Err(exceptions::PyValueError::new_err(format!(
                            "arg2 is wrong value. {:?}",
                            arg2
                        )));
                    }
                } else {
                    0
                };
                Ok(Self(
                    RustDecimal::from_i128_with_scale(num, scale),
                    *DECIMAL_VERSION_HASH,
                ))
                //if ob.len() == 1 {
                //    let item = ob.get_item(0).unwrap();
                //    let py_string = item.cast_as::<PyString>();
                //    if let Ok(content) = py_string {
                //        let rust_str: &str = &content.to_str().unwrap();
                //        let result = RustDecimal::from_str(rust_str);
                //        return match result {
                //            Ok(v) => Ok(Self(v, *DECIMAL_VERSION_HASH)),
                //            Err(_) => Err(exceptions::PyValueError::new_err(format!(
                //                "Input String is wrong value. {}",
                //                rust_str
                //            ))),
                //        };
                //    }
                //    let py_int = item.cast_as::<PyInt>();
                //    if let Ok(content) = py_int {
                //        let num: i128 = content.extract().unwrap();
                //        return Ok(Self(
                //            RustDecimal::from_i128_with_scale(num, 0),
                //            *DECIMAL_VERSION_HASH,
                //        ));
                //    }
                //    let py_float = item.cast_as::<PyFloat>();
                //    if let Ok(content) = py_float {
                //        let num: f64 = content.extract().unwrap();
                //        return Ok(Self(
                //            RustDecimal:: from_f64_retain(num).expect("Failed to load from float value"),
                //            *DECIMAL_VERSION_HASH,
                //        ));
                //    }
                //    Err(exceptions::PyValueError::new_err(format!(
                //        "Input is wrong value. {:?}",
                //        item
                //    )))
                //} else if ob.len() == 2 {
                //    let item0 = ob.get_item(0).unwrap();
                //    let py_num = item0.cast_as::<PyInt>();
                //    let num: i128 = if let Ok(num) = py_num {
                //        num.extract().unwrap()
                //    } else {
                //        return Err(exceptions::PyValueError::new_err(format!(
                //            "First Input is wrong value. {:?}",
                //            item0
                //        )));
                //    };
                //    let item1 = ob.get_item(1).unwrap();
                //    let py_scale = item1.cast_as::<PyInt>();
                //    let scale: u32 = if let Ok(scale) = py_scale {
                //        scale.extract().unwrap()
                //    } else {
                //        return Err(exceptions::PyValueError::new_err(format!(
                //            "First Input is wrong value. {:?}",
                //            item1
                //        )));
                //    };
                //    Ok(Self(
                //        RustDecimal::from_i128_with_scale(num, scale),
                //        *DECIMAL_VERSION_HASH,
                //    ))
                //} else {
                //    Err(exceptions::PyValueError::new_err(format!(
                //        "Input Value is not acceptable {:?}",
                //        ob
                //    )))
                //}
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

            fn __add__(&self, other: &Decimal) -> PyResult<Decimal> {
                Ok((self.0 + other.0).into())
            }

            fn __sub__(&self, other: &Decimal) -> PyResult<Decimal> {
                Ok((self.0 - other.0).into())
            }

            fn __mul__(&self, other: &Decimal) -> PyResult<Decimal> {
                Ok((self.0 * other.0).into())
            }

            fn __truediv__(&self, other: &Decimal) -> PyResult<Decimal> {
                Ok((self.0 / other.0).into())
            }

            fn __floordiv__(&self, other: &Decimal) -> PyResult<Decimal> {
                Ok((self.0 / other.0).into())
            }

            fn __neg__(&self) -> PyResult<Decimal> {
                Ok((-self.0).into())
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
        impl Decimal {
            pub fn from_i128_with_scale<'p>(num: i128, scale: u32) -> Decimal {
                Self(
                    RustDecimal::from_i128_with_scale(num, 0),
                    *DECIMAL_VERSION_HASH,
                )
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
