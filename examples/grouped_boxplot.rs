use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

fn manual_melt(df: &DataFrame, value_vars: &[&str], id_vars: &[&str]) -> PolarsResult<DataFrame> {
    let mut melted_dfs: Vec<DataFrame> = Vec::new();
    // Assume id_vars only has one column: "species"
    let id_col = id_vars[0];

    for &col_name in value_vars {
        // 1. Select ID and current value column (dereference col_name)
        let mut sub_df = df.select([id_col, col_name])?;
        // 2. Create "variable" column with repeated column name
        let variable_series = Series::new("variable".into(), vec![col_name; sub_df.height()]);
        sub_df.with_column(variable_series)?; // Add variable column
        // 3. Rename value column to "value"
        sub_df.rename(col_name, "value".into())?;
        // 4. Reorder columns to (species, variable, value)
        sub_df = sub_df.select([id_col, "variable", "value"])?;
        melted_dfs.push(sub_df);
    }
    // 5. Stack all DataFrames vertically
    let mut final_df = melted_dfs.remove(0);
    for df_to_stack in melted_dfs {
        final_df = final_df.vstack(&df_to_stack)?;
    }

    Ok(final_df)
}

fn main() -> Result<(), Box<dyn Error>> {
    let df = load_dataset("iris")?;
    let df_melted = manual_melt(
        &df,
        &["sepal_length", "sepal_width", "petal_length", "petal_width"],
        &["species"],
    )?;
    println!("{}", &df_melted);

    Chart::build(&df_melted)?
        .mark_boxplot()
        .encode((x("variable"), y("value"), color("species")))?
        .into_layered()
        .save("./examples/grouped_boxplot.svg")?;

    Ok(())
}
