use std::fmt;

pub(crate) struct FileName<'a>(&'a str);

impl<'a> FileName<'a> {
    pub(crate) fn as_str(&self) -> &'a str {
        self.0
    }
}

#[derive(Debug)]
pub(crate) struct InvalidFileName;

impl<'a> TryFrom<&'a str> for FileName<'a> {
    type Error = InvalidFileName;

    fn try_from(name: &'a str) -> Result<Self, Self::Error> {
        let mut bytes = name.bytes();
        let Some(first) = bytes.next() else {
            return Err(InvalidFileName);
        };
        if (first.is_ascii_alphabetic() || first == b'_')
            && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        {
            Ok(Self(name))
        } else {
            Err(InvalidFileName)
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct AllowedFiles(Vec<String>);

impl fmt::Debug for AllowedFiles {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl TryFrom<Vec<String>> for AllowedFiles {
    type Error = InvalidFileName;

    fn try_from(paths: Vec<String>) -> Result<Self, Self::Error> {
        for path in &paths {
            FileName::try_from(path.as_str())?;
        }
        Ok(Self(paths))
    }
}

impl AllowedFiles {
    pub(crate) fn as_slice(&self) -> &[String] {
        &self.0
    }

    pub(crate) fn names(&self) -> impl Iterator<Item = FileName<'_>> {
        self.0.iter().map(|path| FileName(path.as_str()))
    }
}
