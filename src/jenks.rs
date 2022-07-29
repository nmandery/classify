use rand::prelude::*;
use rand::rngs::StdRng;

use std::collections::HashSet;

use crate::utilities::{
    breaks_to_classification, create_unique_val_mapping, unique_to_normal_breaks,
};
use crate::utilities::{Classification, UniqueVal};

/// Returns a Classification object following the Jenks Natural Breaks algorithm given the desired number of categories and one-dimensional f64 data
///
/// # Arguments
///
/// * `num_bins` - A reference to an integer (u64) representing the desired number of bins
/// * `data` - A reference to a vector of unsorted data points (f64) to generate breaks for
///
/// # Examples
///
/// ```
/// use classify::get_jenks_classification;
/// use classify::{Classification, Bin};
/// use rand::prelude::*;
/// use rand::rngs::StdRng;
///
/// let data: Vec<f64> = vec![1.0, 2.0, 4.0, 5.0, 7.0, 8.0];
/// let num_bins = 3;
///
/// let result: Classification = get_jenks_classification(&num_bins, &data);
/// let expected: Classification = Classification {bins: vec![
///     Bin{bin_start: 1.0, bin_end: 4.0, count: 2},
///     Bin{bin_start: 4.0, bin_end: 7.0, count: 2},
///     Bin{bin_start: 7.0, bin_end: 8.0, count: 2}]
/// };
///
/// assert!(result == expected);
/// ```
pub fn get_jenks_classification(num_bins: &usize, data: &Vec<f64>) -> Classification {
    let breaks: Vec<f64> = get_jenks_breaks(num_bins, data);
    breaks_to_classification(&breaks, data)
}

/// Returns a vector of breaks generated through the Jenks Natural Breaks algorithm given the desired number of bins and a dataset
///
/// # Arguments
///
/// * `num_bins` - The desired number of bins
/// * `data` - A reference to a vector of unsorted data points (f64) to generate breaks for
///
/// # Examples
///
/// ```
/// use classify::get_jenks_breaks;
/// use rand::prelude::*;
/// use rand::rngs::StdRng;
///
/// let data: Vec<f64> = vec![1.0, 2.0, 4.0, 5.0, 7.0, 8.0];
/// let num_bins = 3;
///
/// let result: Vec<f64> = get_jenks_breaks(&num_bins, &data);
///
/// assert_eq!(result, vec![4.0, 7.0]);
/// ```
pub fn get_jenks_breaks(num_bins: &usize, data: &Vec<f64>) -> Vec<f64> {
    let num_vals = data.len();

    let mut sorted_data: Vec<f64> = vec![];
    for item in data.iter().take(num_vals) {
        sorted_data.push(*item);
    }
    sorted_data.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mut unique_val_map: Vec<UniqueVal> = vec![];
    create_unique_val_mapping(&mut unique_val_map, &sorted_data);

    let num_unique_vals = unique_val_map.len();
    let true_num_bins = std::cmp::min(&num_unique_vals, num_bins);

    let gssd = calc_gssd(&sorted_data);

    let mut rand_breaks: Vec<usize> = vec![0_usize; true_num_bins - 1];
    let mut best_breaks: Vec<usize> = vec![0_usize; true_num_bins - 1];
    let mut unique_rand_breaks: Vec<usize> = vec![0_usize; true_num_bins - 1];

    let mut max_gvf: f64 = 0.0;

    let c = 5000 * 2200 * 4;
    let mut permutations = c / num_vals;
    if permutations < 10 {
        permutations = 10
    }
    if permutations > 10000 {
        permutations = 10000
    }
    println!("permutations: {}", permutations);

    let mut pseudo_rng = StdRng::seed_from_u64(123456789);

    for _ in 0..permutations {
        pick_rand_breaks(&mut unique_rand_breaks, &num_unique_vals, &mut pseudo_rng);
        unique_to_normal_breaks(&unique_rand_breaks, &unique_val_map, &mut rand_breaks);
        let new_gvf: f64 = calc_gvf(&rand_breaks, &sorted_data, &gssd);
        if new_gvf > max_gvf {
            max_gvf = new_gvf;
            best_breaks[..rand_breaks.len()].copy_from_slice(&rand_breaks[..]);
        }
    }

    let mut nat_breaks: Vec<f64> = vec![];
    nat_breaks.resize(best_breaks.len(), 0.0);
    for i in 0..best_breaks.len() {
        nat_breaks[i] = sorted_data[best_breaks[i]];
    }
    println!("Breaks: {:#?}", nat_breaks);

    nat_breaks
}

/// Populates a vector with a set of breaks as unique random integers that are valid indices within the dataset given the number of data points and an RNG
///
/// # Arguments
///
/// * `breaks` - A mutable reference to an empty vector of breaks whose length is taken to be the desired number of breaks
/// * `num_vals` - A reference to the number of data points
/// * `rng` - A mutable reference to a seedable random number generator (RNG) from the "rand" crate
pub fn pick_rand_breaks(breaks: &mut Vec<usize>, num_vals: &usize, rng: &mut StdRng) {
    let num_breaks = breaks.len();
    if num_breaks > num_vals - 1 {
        return;
    }

    let mut set = HashSet::new();
    while set.len() < num_breaks {
        set.insert(rng.gen_range(1..*num_vals));
    }
    let mut set_iter = set.iter();
    for item in breaks.iter_mut().take(set_iter.len()) {
        *item = *set_iter.next().unwrap();
    }
    breaks.sort_unstable();
}

/// Calculates goodness of variance fit (GVF) for a particular set of breaks on a dataset
///
/// # Arguments
///
/// * `breaks` - A reference to a vector (usize) of break indices (sorted, ascending)
/// * `vals` - A reference to a vector (f64) of data points (sorted, ascending)
/// * `gssd` - A reference to the global sum of squared deviations (GSSD)
pub fn calc_gvf(breaks: &Vec<usize>, vals: &Vec<f64>, gssd: &f64) -> f64 {
    let num_vals = vals.len();
    let num_bins = breaks.len() + 1;
    let mut tssd: f64 = 0.0;
    for i in 0..num_bins {
        let lower = if i == 0 { 0 } else { breaks[i - 1] };
        let upper = if i == num_bins - 1 {
            num_vals
        } else {
            breaks[i]
        };

        let mut mean: f64 = 0.0;
        let mut ssd: f64 = 0.0;
        for item in vals.iter().take(upper).skip(lower) {
            mean += item;
        }
        mean /= (upper - lower) as f64;
        for item in vals.iter().take(upper).skip(lower) {
            ssd += (item - mean) * (item - mean)
        }
        tssd += ssd;
    }
    1.0 - (tssd / gssd)
}

/// Calculates global sum of squared deviations (GSSD) for a particular dataset
///
/// # Arguments
///
/// * `data` - A reference to a vector (f64) of data points (sorted, ascending)
pub fn calc_gssd(data: &Vec<f64>) -> f64 {
    let num_vals = data.len();
    let mut mean = 0.0;
    let mut max_val: f64 = data[0];
    for item in data.iter().take(num_vals) {
        let val = *item;
        if val > max_val {
            max_val = val
        }
        mean += val;
    }
    mean /= num_vals as f64;

    let mut gssd: f64 = 0.0;
    for item in data.iter().take(num_vals) {
        let val = *item;
        gssd += (val - mean) * (val - mean);
    }

    gssd
}
