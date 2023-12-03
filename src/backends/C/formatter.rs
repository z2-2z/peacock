use std::io::{Write, BufWriter};

pub struct CFormatter<T: Write> {
    stream: BufWriter<T>,
    indentation: usize,
}

impl<T> CFormatter<T>
where
    T: Write,
{
    pub fn new(writer: T) -> Self {
        Self {
            stream: BufWriter::new(writer),
            indentation: 0,
        }
    }
    
    pub fn indent(&mut self) {
        self.indentation += 4;
    }
    
    pub fn unindent(&mut self) {
        if self.indentation > 0 {
            self.indentation -= 4;
        }
    }
    
    pub fn write<S: AsRef<str>>(&mut self, line: S) {
        writeln!(&mut self.stream, "{:width$}{}", "", line.as_ref(), width = self.indentation).expect("Could not write to outfile");
    }
    
    pub fn blankline(&mut self) {
        writeln!(&mut self.stream).expect("Could not write to outfile");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::stdout;
    
    #[test]
    fn test_formatter() {
        let mut fmt = CFormatter::new(stdout());
        fmt.write("asdf {");
        fmt.indent();
        fmt.blankline();
        fmt.write("yeehaw");
        fmt.blankline();
        fmt.unindent();
        fmt.write("}");
    }
}
