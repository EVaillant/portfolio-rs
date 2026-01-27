use spreadsheet_ods::{CellStyleRef, Sheet, Value};

pub trait TableBuilderStyleResolver {
    fn get_style(&self, header: &str, value: &Value) -> Option<CellStyleRef>;
}

type TableBuilderValue<'a, T, K> = Box<dyn Fn(&T, &K, &mut Sheet, u32, u32) + 'a>;

pub struct TableBuilder<'a, T, K: TableBuilderStyleResolver> {
    headers: Vec<String>,
    values: Vec<TableBuilderValue<'a, T, K>>,
}

impl<'a, T, K: TableBuilderStyleResolver> TableBuilder<'a, T, K> {
    pub fn new() -> Self {
        Self {
            headers: Default::default(),
            values: Default::default(),
        }
    }

    pub fn add<G, V, N>(&mut self, name: N, getter: G) -> &mut Self
    where
        G: Fn(&T) -> V + 'a,
        V: Into<Value>,
        N: Into<String>,
    {
        let name = name.into();
        self.headers.push(name.to_string());
        self.values.push(Box::new(
            move |data: &T, resolver: &K, sheet: &mut Sheet, row: u32, col: u32| {
                let value: Value = getter(data).into();
                if let Some(style) = resolver.get_style(&name, &value) {
                    sheet.set_styled_value(row, col, value, &style);
                } else {
                    sheet.set_value(row, col, value);
                }
            },
        ));
        self
    }

    pub fn add_optional<G, V, N>(&mut self, name: N, getter: G) -> &mut Self
    where
        G: Fn(&T) -> Option<V> + 'a,
        V: Into<Value>,
        N: Into<String>,
    {
        let name = name.into();
        self.headers.push(name.to_string());
        self.values.push(Box::new(
            move |data: &T, resolver: &K, sheet: &mut Sheet, row: u32, col: u32| {
                if let Some(value) = getter(data).map(|item| item.into()) {
                    if let Some(style) = resolver.get_style(&name, &value) {
                        sheet.set_styled_value(row, col, value, &style);
                    } else {
                        sheet.set_value(row, col, value);
                    }
                }
            },
        ));
        self
    }

    pub fn write<I>(&self, sheet: &mut Sheet, resolver: &K, row: u32, col: u32, inputs: I) -> u32
    where
        I: Iterator<Item = T>,
    {
        for (position, header) in self.headers.iter().enumerate() {
            sheet.set_value(row, col + position as u32, header);
        }

        let mut row = row + 1;
        for input in inputs {
            self.write_line(sheet, resolver, row, col, &input);
            row += 1;
        }
        row
    }

    pub fn write_line(&self, sheet: &mut Sheet, resolver: &K, row: u32, col: u32, input: &T) {
        for (shift_value, value) in self.values.iter().enumerate() {
            (value)(input, resolver, sheet, row, col + shift_value as u32);
        }
    }

    pub fn write_reversed<I>(
        &self,
        sheet: &mut Sheet,
        resolver: &K,
        row: u32,
        col: u32,
        inputs: I,
    ) -> u32
    where
        I: Iterator<Item = T>,
    {
        for (position, header) in self.headers.iter().enumerate() {
            sheet.set_value(row + position as u32, col, header);
        }

        for (position, input) in inputs.into_iter().enumerate() {
            self.write_reversed_line(sheet, resolver, row, position as u32 + col + 1, &input);
        }
        self.headers.len() as u32
    }

    pub fn write_reversed_line(
        &self,
        sheet: &mut Sheet,
        resolver: &K,
        row: u32,
        col: u32,
        input: &T,
    ) {
        for (shift_value, value) in self.values.iter().enumerate() {
            (value)(input, resolver, sheet, row + shift_value as u32, col);
        }
    }
}
