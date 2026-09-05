//! DTO 宏：为数据传输结构体生成 getter / setter / Display / Builder。
//!
//! 项目约定结构体字段**不使用 pub**，由访问器暴露，构造走建造者链式调用。
//! 手写这些样板对 8 个 DTO、73 个字段来说不现实，因此统一由本宏生成。
//!
//! 生成内容（以 `Todo { id: String }` 为例）：
//! - `fn id(&self) -> &String`：getter，与字段同名；
//! - `fn set_id(&mut self, value: String) -> &mut Self`：setter，返回 `&mut Self` 以便链式；
//! - `impl Display`：形如 `Todo { id: "x", … }`，对应 Java 的 toString；
//! - `TodoBuilder` + `Todo::builder()`：链式构造，`build()` 校验字段是否齐全。
//!
//! 依赖 `paste`：`macro_rules!` 不能拼接标识符（生成 `set_id` 需要
//! 不稳定的 `concat_idents!`），`paste::paste!` 是唯一的稳定做法。
//!
//! 生成的访问器都带 `#[allow(dead_code)]`：DTO 的读写往往不对称——
//! 统计类结构只被构造后序列化给前端，Rust 侧既不读也不写它的字段。
//! 用 `pub` 字段时 serde 直接访问字段、不触发 dead_code；改成私有字段 + 访问器后，
//! 这些访问器就成了没有调用方的样板。宏无法区分哪个 DTO 会用到，故统一放行。
//! 代价是拼错名字的访问器不会被 dead_code 检查出来——这是宏化封装的固有取舍。
//!
//! Builder 的字段类型是 `Option<字段类型>`——**包括本身就是 `Option<T>` 的字段**，
//! 此时为 `Option<Option<T>>`：未设置报「缺少字段」，显式设 `None` 才通过。
//! 这样 `build()` 的必填校验与原先的结构体字面量等价（字面量也必须列全字段）。

/// 定义一个 DTO：私有字段 + 访问器 + Display + Builder。
///
/// 用法与普通结构体声明一致，字段**不写** `pub`：
/// ```ignore
/// dto! {
///     /// 文档注释与 derive 都照常透传。
///     #[derive(Debug, Clone, Serialize, Deserialize)]
///     pub struct Demo {
///         /// 字段注释也会保留。
///         id: String,
///         #[serde(default)]
///         count: i64,
///     }
/// }
/// ```
#[macro_export]
macro_rules! dto {
    (
        $(#[$struct_meta:meta])*
        pub struct $name:ident {
            $(
                $(#[$field_meta:meta])*
                $field:ident : $ty:ty
            ),* $(,)?
        }
    ) => {
        $(#[$struct_meta])*
        pub struct $name {
            $(
                $(#[$field_meta])*
                $field: $ty,
            )*
        }

        // 整个 impl 块都放进 paste!：`paste!` 不能出现在返回类型位置，
        // 必须包住整个 item 才能同时用上 [<...>] 拼接与类型引用。
        paste::paste! {
            impl $name {
                /// 取建造者，链式设置字段后 `build()`。
                pub fn builder() -> [<$name Builder>] {
                    [<$name Builder>]::default()
                }

                $(
                    #[doc = concat!("读取 `", stringify!($field), "`。")]
                    // 与 setter 同理：只序列化给前端的 DTO（统计类）在 Rust 侧不读字段，
                    // 访问器没有调用方。宏无法区分哪个 DTO 会用到，统一放行。
                    #[allow(dead_code)]
                    pub fn $field(&self) -> &$ty {
                        &self.$field
                    }
                )*

                $(
                    #[doc = concat!("写入 `", stringify!($field), "`，返回 `&mut Self` 以便链式调用。")]
                    // 读写不对称是 DTO 的常态：统计类结构只被读取，没有 setter 调用方。
                    // 宏是通用设施，不该为了消告警逼调用方去用不需要的 setter。
                    #[allow(dead_code)]
                    pub fn [<set_ $field>](&mut self, value: $ty) -> &mut Self {
                        self.$field = value;
                        self
                    }
                )*
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, concat!(stringify!($name), " {{"))?;
                let mut first = true;
                $(
                    if !first {
                        write!(f, ",")?;
                    }
                    first = false;
                    write!(f, concat!(" ", stringify!($field), ": {:?}"), self.$field)?;
                )*
                // first 在无字段的结构体上不会被读取，显式忽略以免告警。
                let _ = first;
                write!(f, " }}")
            }
        }

        paste::paste! {
            #[doc = concat!("`", stringify!($name), "` 的建造者，字段全部设置后调用 `build()`。")]
            #[derive(Default)]
            pub struct [<$name Builder>] {
                $($field: Option<$ty>,)*
            }

            impl [<$name Builder>] {
                $(
                    #[doc = concat!("设置 `", stringify!($field), "`。")]
                    // 建造者方法按值消费 self 以支持链式；字段名恰好以 is_ 开头时
                    // 会触发 wrong_self_convention（它期望 is_* 借用 self），
                    // 但那条约定针对判定方法，不适用于建造者。
                    #[allow(clippy::wrong_self_convention)]
                    pub fn $field(mut self, value: $ty) -> Self {
                        self.$field = Some(value);
                        self
                    }
                )*

                /// 校验字段齐全并构造实例；缺字段时返回其名称，便于定位漏设的调用点。
                pub fn build(self) -> Result<$name, String> {
                    Ok($name {
                        $(
                            $field: self.$field.ok_or_else(|| {
                                format!(
                                    concat!("构造 ", stringify!($name), " 缺少字段: ", stringify!($field))
                                )
                            })?,
                        )*
                    })
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    // 宏在 crate 根导出，测试里按绝对路径引用。
    crate::dto! {
        /// 验证宏用的最小结构体。
        #[derive(Debug, Clone, PartialEq)]
        pub struct Demo {
            /// 标识。
            id: String,
            count: i64,
            tag: Option<String>,
        }
    }

    #[test]
    fn builder_builds_and_getters_read() {
        let demo = Demo::builder()
            .id("x".into())
            .count(3)
            .tag(None)
            .build()
            .unwrap();
        assert_eq!(demo.id(), "x");
        assert_eq!(*demo.count(), 3);
        assert_eq!(*demo.tag(), None);
    }

    #[test]
    fn builder_reports_missing_field_by_name() {
        // 漏设 count：错误信息要带字段名，否则定位漏设的调用点很费劲。
        let error = Demo::builder()
            .id("x".into())
            .tag(None)
            .build()
            .unwrap_err();
        assert!(error.contains("count"), "实际错误: {error}");
    }

    #[test]
    fn setters_are_chainable() {
        let mut demo = Demo::builder()
            .id("a".into())
            .count(1)
            .tag(None)
            .build()
            .unwrap();
        demo.set_id("b".into()).set_count(2);
        assert_eq!(demo.id(), "b");
        assert_eq!(*demo.count(), 2);
    }

    #[test]
    fn display_lists_all_fields() {
        let demo = Demo::builder()
            .id("x".into())
            .count(3)
            .tag(Some("t".into()))
            .build()
            .unwrap();
        let text = demo.to_string();
        assert!(text.starts_with("Demo {"), "实际: {text}");
        for field in ["id", "count", "tag"] {
            assert!(text.contains(field), "缺字段 {field}: {text}");
        }
    }
}
