use charton::prelude::*;
use std::error::Error;

#[test]
fn test_select_columns_for_debug() -> Result<(), Box<dyn Error>> {
    let ds = Dataset::new()
        .with_column("a", vec![1, 2, 3])?
        .with_column("b", vec![4, 5, 6])?
        .with_column("c", vec![7, 8, 9])?;

    let subset = ds.select(&["b", "c"])?;

    assert_eq!(subset.height(), 3);
    assert_eq!(subset.width(), 2);
    assert_eq!(
        subset.get_column_names(),
        vec!["b".to_string(), "c".to_string()]
    );

    Ok(())
}
