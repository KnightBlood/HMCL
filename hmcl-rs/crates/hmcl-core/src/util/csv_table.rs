//! A table of text that is kept as comma separated values.
//!
//! The launcher writes the addons the player has not updated yet into a
//! file of comma separated values, so the player can open it in a sheet.
//!
//! The table keeps every cell it was given, even when a whole row or a
//! whole column is left out, so the file of the table looks the same as
//! the one of the Java launcher.

use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

/// A table of text that is kept as comma separated values.
///
/// A cell that was never written is empty, and a cell that was written as
/// an empty text is empty as well, so the two cannot be told apart.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CsvTable {
    /// The rows of the table, each of them a list of the cells in it.
    ///
    /// A row that was never written is kept as `None`, so it is written as
    /// a row of empty cells.
    rows: Vec<Option<Vec<String>>>,
    /// The number of the columns the table has.
    column_count: usize,
}

impl CsvTable {
    /// Creates a table of no cell at all.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of the columns the table has.
    pub fn column_count(&self) -> usize {
        self.column_count
    }

    /// Returns the number of the rows the table has.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Returns the text of the cell.
    ///
    /// A cell that was never written, and a cell outside of the table, are
    /// both empty.
    pub fn get(&self, column: usize, row: usize) -> &str {
        self.rows
            .get(row)
            .and_then(|row| row.as_ref())
            .and_then(|row| row.get(column))
            .map_or("", String::as_str)
    }

    /// Writes the text into the cell, and grows the table to hold it.
    pub fn set(&mut self, column: usize, row: usize, text: &str) {
        if row >= self.rows.len() {
            self.rows.resize_with(row + 1, || None);
        }

        let cells = self.rows[row].get_or_insert_with(Vec::new);
        if column >= cells.len() {
            cells.resize(column + 1, String::new());
        }
        cells[column] = text.to_string();

        self.column_count = self.column_count.max(column + 1);
    }

    /// Writes the table into the file, in the UTF-8 encoding.
    pub fn write_to_file(&self, path: &Path) -> io::Result<()> {
        fs::write(path, self.to_string())
    }

    /// Writes the table into the writer.
    ///
    /// A text that holds a quote, a comma or a line break is written inside
    /// quotes, and every quote in it is doubled, like in the Java launcher.
    fn write_to_writer(&self, out: &mut String) {
        for row in &self.rows {
            let cells = row.as_ref();

            for column in 0..self.column_count {
                let text = cells.and_then(|cells| cells.get(column));

                if let Some(text) = text {
                    if !text.contains(['"', ',', '\n', '\r']) {
                        out.push_str(text);
                    } else {
                        out.push('"');
                        out.push_str(&text.replace('"', "\"\""));
                        out.push('"');
                    }
                }

                if column + 1 < self.column_count {
                    out.push(',');
                }
            }

            out.push('\n');
        }
    }
}

impl fmt::Display for CsvTable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut written = String::new();
        self.write_to_writer(&mut written);
        formatter.write_str(&written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cells_of_a_new_table_are_empty() {
        let mut table = CsvTable::new();

        assert_eq!(table.column_count(), 0);
        assert_eq!(table.row_count(), 0);
        assert_eq!(table.to_string(), "");

        table.set(0, 0, "test1");
        assert_eq!(table.column_count(), 1);
        assert_eq!(table.row_count(), 1);

        table.set(4, 0, "test2");
        assert_eq!(table.column_count(), 5);
        assert_eq!(table.row_count(), 1);

        table.set(2, 1, "test3");
        assert_eq!(table.column_count(), 5);
        assert_eq!(table.row_count(), 2);

        assert_eq!(table.get(0, 0), "test1");
        assert_eq!(table.get(1, 0), "");
        assert_eq!(table.get(0, 1), "");
        assert_eq!(table.get(4, 0), "test2");
        assert_eq!(table.get(2, 1), "test3");

        // A cell outside of the table is empty as well.
        assert_eq!(table.get(0, 2), "");
        assert_eq!(table.get(5, 0), "");
    }

    #[test]
    fn the_table_is_written_the_way_java_writes_it() {
        let mut table = CsvTable::new();
        table.set(0, 0, "a");
        table.set(1, 0, "b");
        table.set(3, 0, "c");
        table.set(0, 2, "a,b");
        table.set(1, 2, "c");
        table.set(3, 2, "d\"e\n");

        assert_eq!(table.to_string(), "a,b,,c\n,,,\n\"a,b\",c,,\"d\"\"e\n\"\n");
    }

    #[test]
    fn the_table_is_written_into_a_file() {
        let directory = std::env::temp_dir().join(format!(
            "hmcl-rs-csv-table-{}-{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&directory).unwrap();

        let mut table = CsvTable::new();
        table.set(0, 0, "a,b");
        table.set(1, 0, "c");

        let path = directory.join("table.csv");
        table.write_to_file(&path).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), table.to_string());

        std::fs::remove_dir_all(&directory).unwrap();
    }
}
