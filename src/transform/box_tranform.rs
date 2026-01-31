use crate::chart::Chart;
use crate::error::ChartonError;
use crate::mark::boxplot::MarkBoxplot;
use polars::prelude::*;

impl Chart<MarkBoxplot> {
    /// Performs high-performance statistical aggregation for Box Plots.
    ///
    /// This transform supports grouped box plots with "Dodge" alignment:
    /// 1. Groups by X and Color.
    /// 2. Uses a Join to assign fixed global indices to colors (ensuring gaps for missing groups).
    /// 3. Computes 5-number summary and identifies outliers/whiskers.
    pub(crate) fn transform_boxplot_data(mut self) -> Result<Self, ChartonError> {
        let x_name = &self.encoding.x.as_ref().unwrap().field;
        let y_name = &self.encoding.y.as_ref().unwrap().field;

        // --- STEP 1: IDENTIFY GROUPING COLUMNS ---
        let mut group_cols = vec![col(x_name)];
        let mut color_field_name: Option<String> = None;

        if let Some(color_enc) = &self.encoding.color {
            group_cols.push(col(&color_enc.field));
            color_field_name = Some(color_enc.field.clone());
        }

        // --- STEP 2: INITIAL AGGREGATION ---
        let mut df_stats = self.data.df.clone().lazy()
            .group_by(group_cols)
            .agg([
                col(y_name).quantile(lit(0.25), QuantileMethod::Linear).alias("q1"),
                col(y_name).median().alias("median"),
                col(y_name).quantile(lit(0.75), QuantileMethod::Linear).alias("q3"),
                col(y_name).implode().alias("raw_values"), 
            ])
            .collect()?;

        // --- STEP 3: CALCULATE GLOBAL DODGE PARAMETERS ---
        // We calculate fixed indices for colors to ensure boxes stay in their "slots"
        let mut groups_count = 1.0;
        
        if let Some(ref color_field) = color_field_name {
            // Get unique colors from the source data and sort them for consistent indexing
            let unique_colors = self.data.df.column(color_field)?
                .unique()?
                .sort(SortOptions {
                    descending: false,
                    nulls_last: true,
                    multithreaded: true,
                    maintain_order: false,
                    limit: None,
                })?;
            
            groups_count = unique_colors.len() as f64;
            
            // Create a mapping DataFrame: [ColorName, sub_idx]
            let sub_idx_series = Series::new(
                "sub_idx".into(), 
                (0..unique_colors.len()).map(|i| i as f64).collect::<Vec<f64>>()
            );
            let map_df = DataFrame::new(vec![
                unique_colors.with_name(color_field.clone().into()), 
                sub_idx_series.into()
            ])?;

            // Join the stats with the mapping table to assign sub_idx to every row
            df_stats = df_stats.left_join(&map_df, [color_field], [color_field])?;
        } else {
            // No color: every box is in slot 0, and there's only 1 box per X
            df_stats.with_column(Series::new("sub_idx".into(), vec![0.0; df_stats.height()]))?;
        }

        // --- STEP 4: REFINED STATS CALCULATION (WHISKERS & OUTLIERS) ---
        let q1_col = df_stats.column("q1")?.f64()?;
        let q3_col = df_stats.column("q3")?.f64()?;
        let raw_list_col = df_stats.column("raw_values")?.list()?;

        let mut whisker_mins = Vec::with_capacity(df_stats.height());
        let mut whisker_maxs = Vec::with_capacity(df_stats.height());
        let mut outliers_list: Vec<Series> = Vec::with_capacity(df_stats.height());

        for i in 0..df_stats.height() {
            let q1 = q1_col.get(i).unwrap();
            let q3 = q3_col.get(i).unwrap();
            let iqr = q3 - q1;
            let lower_bound = q1 - 1.5 * iqr;
            let upper_bound = q3 + 1.5 * iqr;

            let values_series = raw_list_col.get_as_series(i).unwrap();
            let values_f64 = values_series.f64()?;

            let mut group_min = q1; 
            let mut group_max = q3; 
            let mut group_outliers = Vec::new();

            for val_opt in values_f64.into_iter().flatten() {
                if val_opt < lower_bound || val_opt > upper_bound {
                    group_outliers.push(val_opt);
                } else {
                    if val_opt < group_min { group_min = val_opt; }
                    if val_opt > group_max { group_max = val_opt; }
                }
            }
            
            whisker_mins.push(group_min);
            whisker_maxs.push(group_max);
            outliers_list.push(Series::new("outlier".into(), group_outliers));
        }

        // --- STEP 5: FINAL ASSEMBLY ---
        df_stats.with_column(Series::new("min".into(), whisker_mins))?;
        df_stats.with_column(Series::new("max".into(), whisker_maxs))?;
        df_stats.with_column(Series::new("outliers".into(), outliers_list))?;
        df_stats.with_column(Series::new("groups_count".into(), vec![groups_count; df_stats.height()]))?;

        self.data.df = df_stats;
        Ok(self)
    }
}