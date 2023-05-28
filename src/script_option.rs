use std::ffi::OsString;

/// 图像内容输入模式
pub enum ImageInput {
    SingleFile, 
    TwoFiles, 
}

#[derive(Clone)]
/// 脚本执行模式
pub enum ScriptOption {
    /// 直接进程执行
    DirectExecute, 
    /// Py 脚本执行，选择 Python 解释器
    PyExecute(Option<OsString>),
}