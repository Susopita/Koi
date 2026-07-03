// Pipeline map -> filter -> reduce con closures que capturan variables
// libres (factor, threshold) por valor, midiendo el costo de closures
// nativas de Rust frente al fat-pointer + struct de entorno de koi.
// Filtro simplificado a `> threshold` (sin chequeo de paridad) para que la
// comparacion con koi/lambda_map.koi sea equivalente -- koi no tiene un
// builtin `mod`.
fn main() {
    let n: i64 = 1_000_000;
    let data: Vec<i64> = (0..n).collect();

    let factor: i64 = 3;
    let double_fn = |x: i64| x * factor;

    let threshold: i64 = 500_000;
    let keep_fn = |x: i64| x > threshold;

    let sum: i64 = data
        .iter()
        .map(|&x| double_fn(x))
        .filter(|&x| keep_fn(x))
        .sum();

    println!("{}", sum);
}
