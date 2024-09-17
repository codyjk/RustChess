use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};

pub fn generate_opening_book(input_file: &str, out: &mut BufWriter<File>) -> std::io::Result<()> {
    let file = File::open(input_file)?;
    let reader = BufReader::new(file);

    writeln!(out, "\npub fn create_book() -> Book {{")?;
    writeln!(out, "    let mut book = Book::new();")?;

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split(": ").collect();
        if parts.len() == 2 {
            let name = parts[0];
            let moves = parts[1];
            writeln!(out, "    book.add_line(OpeningLine {{
        name: String::from(\"{}\"),
        moves: String::from(\"{}\"),
    }});", name, moves)?;
        }
    }

    writeln!(out, "    book")?;
    writeln!(out, "}}")?;

    out.flush()?;
    Ok(())
}
