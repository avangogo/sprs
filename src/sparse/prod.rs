///! Sparse matrix product

use std::ops::{Deref};
use sparse::csmat::CompressedStorage::{CSC, CSR};
use sparse::csmat::{CsMat};
use num::traits::Num;

pub fn mul_acc_mat_vec_csc<N: Num + Clone + Copy, IStorage: Deref<Target=[usize]>, DStorage: Deref<Target=[N]>>(
    mat: CsMat<N, IStorage, DStorage>, in_vec: &[N], res_vec: &mut[N]) {
    assert!(mat.cols() == in_vec.len(), "Matrix and vector dims must agree");
    assert!(mat.rows() == res_vec.len(), "Matrix and res vector dims must agree");
    assert!(mat.storage_type() == CSC, "Matrix must be in CSC format");

    for (col_ind, vec) in mat.outer_iterator() {
        let multiplier = &in_vec[col_ind];
        for (row_ind, value) in vec.iter() {
            // TODO: unsafe access to value? needs bench
            res_vec[row_ind] =
                res_vec[row_ind] + *multiplier * value;
        }
    }
}

pub fn mul_acc_mat_vec_csr<N: Num + Clone + Copy, IStorage: Deref<Target=[usize]>, DStorage: Deref<Target=[N]>>(
    mat: CsMat<N, IStorage, DStorage>, in_vec: &[N], res_vec: &mut[N]) {
    assert!(mat.cols() == in_vec.len(), "Matrix and vector dims must agree");
    assert!(mat.rows() == res_vec.len(), "Matrix and res vector dims must agree");
    assert!(mat.storage_type() == CSR, "Matrix must be in CSR format");

    for (row_ind, vec) in mat.outer_iterator() {
        for (col_ind, value) in vec.iter() {
            // TODO: unsafe access to value? needs bench
            res_vec[row_ind] =
                res_vec[row_ind] + in_vec[col_ind] * value;
        }
    }
}

/// Perform a matrix multiplication for matrices sharing the same storage order.
///
/// For brevity, this method assumes a CSR storage order, transposition should
/// be used for the CSC-CSC case.
/// Accumulates the result line by line.
///
/// lhs: left hand size matrix
/// rhs: right hand size matrix
/// workspace: used to accumulate the line values. Should be of length
///            rhs.cols()
pub fn csr_mul_csr<N>(lhs: &CsMat<N, &[usize], &[N]>,
                      rhs: &CsMat<N, &[usize], &[N]>,
                      workspace: &mut[Option<N>]
                     ) -> CsMat<N, Vec<usize>, Vec<N>>
where N: Num + Copy {
    let res_rows = lhs.rows();
    let res_cols = rhs.cols();
    assert_eq!(lhs.cols(), rhs.rows());
    assert_eq!(res_cols, workspace.len());
    assert_eq!(lhs.storage_type(), rhs.storage_type());
    assert_eq!(CSR, rhs.storage_type());

    let mut res = CsMat::empty(lhs.storage_type(), res_cols);
    for (_, lvec) in lhs.outer_iterator() {
        // reset the accumulators
        for wval in workspace.iter_mut() {
            *wval = None;
        }
        // accumulate the row values
        for (_, rvec) in rhs.outer_iterator() {
            for (col_ind, lval, rval) in lvec.iter().nnz_zip(rvec.iter()) {
                let wval = &mut workspace[col_ind];
                let prod = lval * rval;
                match wval {
                    &mut None => *wval = Some(prod),
                    &mut Some(ref mut acc) => *acc = *acc + prod
                }
            }
        }
        // compress the row into the resulting matrix
        res = res.append_outer(&workspace);
    }
    assert_eq!(res_rows, res.rows());
    res
}

#[cfg(test)]
mod test {
    use sparse::csmat::{CsMat};
    use sparse::csmat::CompressedStorage::{CSC, CSR};
    use super::{mul_acc_mat_vec_csc, mul_acc_mat_vec_csr};

    #[test]
    fn mul_csc_vec() {
        let indptr: &[usize] = &[0, 2, 4, 5, 6, 7];
        let indices: &[usize] = &[2, 3, 3, 4, 2, 1, 3];
        let data: &[f64] = &[
            0.35310881, 0.42380633, 0.28035896, 0.58082095,
            0.53350123, 0.88132896, 0.72527863];

        let mat = CsMat::from_slices(CSC, 5, 5, indptr, indices, data).unwrap();
        let vector = vec![0.1, 0.2, -0.1, 0.3, 0.9];
        let mut res_vec = vec![0., 0., 0., 0., 0.];
        mul_acc_mat_vec_csc(mat, &vector, &mut res_vec);

        let expected_output =
            vec![ 0., 0.26439869, -0.01803924, 0.75120319, 0.11616419];

        let epsilon = 1e-7; // TODO: get better values and increase precision

        assert!(res_vec.iter().zip(expected_output.iter()).all(
            |(x,y)| (*x-*y).abs() < epsilon));
    }

    #[test]
    fn mul_csr_vec() {
        let indptr: &[usize] = &[0, 3, 3, 5, 6, 7];
        let indices: &[usize] = &[1, 2, 3, 2, 3, 4, 4];
        let data: &[f64] = &[
            0.75672424, 0.1649078, 0.30140296, 0.10358244,
            0.6283315, 0.39244208, 0.57202407];

        let mat = CsMat::from_slices(CSR, 5, 5, indptr, indices, data).unwrap();
        let vector = vec![0.1, 0.2, -0.1, 0.3, 0.9];
        let mut res_vec = vec![0., 0., 0., 0., 0.];
        mul_acc_mat_vec_csr(mat, &vector, &mut res_vec);

        let expected_output =
            vec![0.22527496, 0., 0.17814121, 0.35319787, 0.51482166];

        let epsilon = 1e-7; // TODO: get better values and increase precision

        assert!(res_vec.iter().zip(expected_output.iter()).all(
            |(x,y)| (*x-*y).abs() < epsilon));
    }
}
