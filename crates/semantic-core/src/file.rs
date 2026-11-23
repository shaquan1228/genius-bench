/* Fixture files: immutable initial state, the seed for file and non-file environments.
https://inspect.aisi.org.uk/datasets.html#sample-files
https://docs.harborframework.com/tasks/environment */
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct File {
    pub path: String,
    pub contents: String,
}

impl File {
    pub(crate) fn is_valid_path(path: &str) -> bool {
        let mut bytes = path.bytes();
        let Some(first) = bytes.next() else {
            return false;
        };
        (first.is_ascii_alphabetic() || first == b'_')
            && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    }
}
