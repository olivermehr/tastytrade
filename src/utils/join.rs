use crate::AsSymbol;

pub fn join_symbols_optional(symbols: Option<&[impl AsSymbol]>) -> String {
    if let Some(symbols) = symbols {
        symbols
            .iter()
            .map(|symbol| symbol.as_symbol().0)
            .collect::<Vec<String>>()
            .join(",")
    } else {
        "".to_string()
    }
}

pub fn join_symbols(symbols: &[impl AsSymbol]) -> String {
    symbols
        .iter()
        .map(|symbol| symbol.as_symbol().0)
        .collect::<Vec<String>>()
        .join(",")
}
