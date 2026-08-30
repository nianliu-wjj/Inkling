//! 字段访问器宏：为结构体生成 getter / setter / to_string 方法。
//!
//! 项目约定：结构体字段一律**不使用 `pub` 修饰**，统一通过本宏生成访问器：
//! - `<field>(&self) -> T`：读取字段（Clone 语义）
//! - `set_<field>(&mut self, value: T)`：写入字段
//! - `<field>_to_string(&self) -> String`：字段的调试字符串形式

/// 为结构体生成字段访问器。
///
/// 用法：
/// ```ignore
/// accessors! {
///     #[derive(Clone, Debug)]
///     pub struct Point {
///         x: i32,
///         y: i32,
///     }
/// }
/// // 生成：point.x() / point.set_x(1) / point.x_to_string()
/// ```
#[macro_export]
macro_rules! accessors {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident {
            $($(#[$fmeta:meta])* $field:ident : $t:ty),* $(,)?
        }
    ) => {
        paste::paste! {
            $(#[$meta])*
            $vis struct $name {
                $( $(#[$fmeta])* $field: $t, )*
            }

            #[allow(dead_code)]
            impl $name {
                $(
                    #[doc = "字段读取（返回克隆）"]
                    pub fn [<$field>](&self) -> $t {
                        self.$field.clone()
                    }

                    #[doc = "字段写入"]
                    pub fn [<set_ $field>](&mut self, value: $t) {
                        self.$field = value;
                    }

                    #[doc = "字段的调试字符串形式"]
                    pub fn [<$field _to_string>](&self) -> String {
                        format!("{:?}", self.$field)
                    }
                )*
            }
        }
    };
}
