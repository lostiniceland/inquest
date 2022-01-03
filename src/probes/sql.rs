use std::cmp::max;
use std::fmt::{Display, Formatter};
use std::iter::Iterator;

pub(super) type Columns = Vec<String>;
pub(super) type Rows = Vec<Row>;
pub(super) type Row = Vec<String>;

pub(super) struct Table {
    column_sizes: Vec<usize>,
    columns: Columns,
    rows: Rows,
}

impl<'set> Table {
    pub(super) fn new(columns: Columns, rows: Rows) -> Table {
        let mut column_sizes = Vec::with_capacity(columns.len());
        for column_index in 0..columns.len() {
            let max_column_length = rows
                .iter()
                .map(|row| row.get(column_index).map(|v| v.len()))
                .flatten()
                .max()
                .unwrap();
            column_sizes.push(max(
                max_column_length,
                columns
                    .get(column_index)
                    .map(|column_name| column_name.len())
                    .unwrap(),
            ));
        }
        Table {
            columns,
            rows,
            column_sizes,
        }
    }
}

impl Display for Table {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        fn pad_right(text: &String, length: &usize) -> String {
            let mut result = String::with_capacity(*length);
            result.push_str(text);
            result.push_str(" ".repeat(length - text.len()).as_ref());
            result
        }

        let full_width = self.column_sizes.iter().map(|size| size + 2).sum();

        let mut header = String::with_capacity(full_width);
        for (index, column) in self.columns.iter().enumerate() {
            header.push_str(
                format!(
                    " {} ",
                    pad_right(
                        &column.to_uppercase(),
                        self.column_sizes.get(index).unwrap()
                    )
                    .as_str()
                )
                .as_str(),
            );
        }
        write!(f, "{}\n", header)?;

        let mut rows = String::with_capacity(full_width);
        for row in &self.rows {
            for (index, value) in row.iter().enumerate() {
                rows.push_str(
                    format!(
                        " {} ",
                        pad_right(value, self.column_sizes.get(index).unwrap()).as_str()
                    )
                    .as_str(),
                )
            }
            rows.push_str("\n");
        }
        write!(f, "{}", rows)
    }
}

#[cfg(test)]
mod tests {
    use crate::probes::sql::Table;

    #[test]
    fn table_displayed_correct() {
        // GIVEN
        let data = TestData {
            columns: vec!["header a", "header b", "header c"],
            rows: vec![
                vec!["a1", "b1", "c1"],
                vec!["a2", "longer than header", "c2"],
            ],
        };
        // WHEN
        let result = format!("{}", Table::from(data));
        // THEN
        assert_eq!(
            result,
            r" HEADER A  HEADER B            HEADER C 
 a1        b1                  c1       
 a2        longer than header  c2       
"
        );
    }

    struct TestData {
        // lifetime for test-runtime enough
        columns: Vec<&'static str>,
        rows: Vec<Vec<&'static str>>,
    }

    impl From<TestData> for Table {
        fn from(mock: TestData) -> Self {
            Table::new(
                mock.columns.iter().map(|x| x.to_string()).collect(),
                mock.rows
                    .iter()
                    .map(|row| row.iter().map(|x| x.to_string()).collect())
                    .collect(),
            )
        }
    }
}
