use charton::error::ChartonError;
use charton::prelude::*;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 先准备好数据（不计入时间）
    let ds = get_10k_dataset()?;

    // 2. 开始计时：只计算从 Chart 构建到文件保存的过程
    let start = Instant::now();

    Chart::build(ds)?
        .mark_point()?
        .encode((x("x"), y("y")))?
        .with_title("Performance Test: 10,000 Points")
        .save("stress_test.svg")?;

    let duration = start.elapsed();

    // 3. 输出结果
    println!("--------------------------------------");
    println!("渲染并保存 10,000 个点耗时: {:?}", duration);
    println!("--------------------------------------");

    Ok(())
}

/// 构建一万个点的压力测试数据集
pub fn get_10k_dataset() -> Result<Dataset, ChartonError> {
    let count = 10_000;

    // 1. 使用迭代器生成 X 轴数据 (0.0, 0.01, 0.02 ... 99.99)
    let x: Vec<f64> = (0..count).map(|i| i as f64 * 0.01).collect();

    // 2. 基于 X 生成 Y 轴数据 (简单的正弦波形)
    // 这种数学运算在 Rust 中是指令级优化的，耗时几乎可以忽略
    let y: Vec<f64> = x.iter().map(|&val| val.sin()).collect();

    // 3. 将数据压入 Dataset
    // 这里 Dataset::new() 后的链式调用会触发内存分配
    let ds = Dataset::new().with_column("x", x)?.with_column("y", y)?;

    Ok(ds)
}
