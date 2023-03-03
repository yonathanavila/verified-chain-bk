use super::TensorError;
use crate::tensor::{Tensor, TensorType};
use itertools::Itertools;
pub use std::ops::{Add, Div, Mul, Sub};

/// Matrix multiplies two 2D tensors (and adds an offset).
/// # Arguments
///
/// * `inputs` - A vector of tensors holding in order: input data, affine kernel, convolution bias.
/// # Examples
/// ```
/// use ezkl::tensor::Tensor;
/// use ezkl::tensor::ops::affine;
///
/// let x = Tensor::<i32>::new(
///     Some(&[5, 2, 3, 0, 4, -1, 3, 1, 6, 2, 1, 1]),
///     &[3, 4],
/// ).unwrap();
/// let k = Tensor::<i32>::new(
///     Some(&[2, 1, 2, 1, 1, 1]),
///     &[2, 3],
/// ).unwrap();
/// let b = Tensor::<i32>::new(
///     Some(&[0, 0]),
///     &[2],
/// ).unwrap();
/// let result = affine(&vec![x, k, b]).unwrap();
/// let expected = Tensor::<i32>::new(Some(&[26, 7, 11, 3, 15, 3, 7, 2]), &[2, 4]).unwrap();
/// assert_eq!(result, expected);
/// ```
pub fn affine<T: TensorType + Mul<Output = T> + Add<Output = T>>(
    inputs: &Vec<Tensor<T>>,
) -> Result<Tensor<T>, TensorError> {
    let (mut input, kernel, bias) = (inputs[0].clone(), inputs[1].clone(), inputs[2].clone());
    if (inputs.len() != 3)
        || (bias.dims()[0] != kernel.dims()[0])
        || (input.dims()[0] != kernel.dims()[1])
    {
        return Err(TensorError::DimMismatch("affine".to_string()));
    }

    // does matrix to vector multiplication
    if input.dims().len() == 1 {
        input.reshape(&[input.dims()[0], 1])
    }

    let input_dims = input.dims();
    let kernel_dims = kernel.dims();

    // calculate value of output
    let mut output: Tensor<T> = Tensor::new(None, &[kernel_dims[0], input_dims[1]]).unwrap();

    for i in 0..kernel_dims[0] {
        for j in 0..input_dims[1] {
            let prod = dot(&vec![
                &kernel.get_slice(&[i..i + 1])?,
                &input.get_slice(&[0..input_dims[0], j..j + 1])?,
            ])?;
            output.set(&[i, j], prod[0].clone() + bias[i].clone());
        }
    }
    // does matrix to vector multiplication
    if output.dims()[1] == 1 {
        output.flatten();
    }
    Ok(output)
}

/// Scales and shifts a tensor.
/// Given inputs (x,k,b) computes k*x + b elementwise
/// # Arguments
///
/// * `inputs` - Vector of tensors of length 2
/// # Examples
/// ```
/// use ezkl::tensor::Tensor;
/// use ezkl::tensor::ops::scale_and_shift;
///
/// let x = Tensor::<i32>::new(
///     Some(&[2, 1, 2, 1, 1, 1]),
///     &[2, 3],
/// ).unwrap();
/// let k = Tensor::<i32>::new(
///     Some(&[2, 1, 2, 1, 1, 1]),
///     &[2, 3],
/// ).unwrap();
/// let b = Tensor::<i32>::new(
///     Some(&[2, 1, 2, 1, 1, 1]),
///     &[2, 3],
/// ).unwrap();
/// let result = scale_and_shift(&vec![x, k, b]).unwrap();
/// let expected = Tensor::<i32>::new(Some(&[6, 2, 6, 2, 2, 2]), &[2, 3]).unwrap();
/// assert_eq!(result, expected);
/// ```
pub fn scale_and_shift<T: TensorType + Mul<Output = T> + Add<Output = T>>(
    inputs: &Vec<Tensor<T>>,
) -> Result<Tensor<T>, TensorError> {
    if (inputs.len() != 3)
        || (inputs[1].dims() != inputs[2].dims())
        || (inputs[0].dims() != inputs[1].dims())
    {
        return Err(TensorError::DimMismatch("scale and shift".to_string()));
    }
    let (input, kernel, bias) = (inputs[0].clone(), inputs[1].clone(), inputs[2].clone());
    let mut output: Tensor<T> = input;
    for (i, bias_i) in bias.iter().enumerate() {
        output[i] = kernel[i].clone() * output[i].clone() + bias_i.clone()
    }
    Ok(output)
}

/// Matrix multiplies two 2D tensors.
/// # Arguments
///
/// * `inputs` - Vector of tensors of length 2
/// # Examples
/// ```
/// use ezkl::tensor::Tensor;
/// use ezkl::tensor::ops::matmul;
///
/// let x = Tensor::<i32>::new(
///     Some(&[5, 2, 3, 0, 4, -1, 3, 1, 6, 2, 1, 1]),
///     &[3, 4],
/// ).unwrap();
/// let k = Tensor::<i32>::new(
///     Some(&[2, 1, 2, 1, 1, 1]),
///     &[2, 3],
/// ).unwrap();
/// let result = matmul(&vec![k, x]).unwrap();
/// let expected = Tensor::<i32>::new(Some(&[26, 7, 11, 3, 15, 3, 7, 2]), &[2, 4]).unwrap();
/// assert_eq!(result, expected);
/// ```
pub fn matmul<T: TensorType + Mul<Output = T> + Add<Output = T>>(
    inputs: &Vec<Tensor<T>>,
) -> Result<Tensor<T>, TensorError> {
    let (a, b) = (inputs[0].clone(), inputs[1].clone());
    if (inputs.len() != 2)
        || (a.dims()[a.dims().len() - 1] != b.dims()[a.dims().len() - 2])
        || (a.dims()[0..a.dims().len() - 2] != b.dims()[0..a.dims().len() - 2])
    {
        return Err(TensorError::DimMismatch("matmul".to_string()));
    }

    let mut dims = Vec::from(&a.dims()[0..a.dims().len() - 2]);
    dims.push(a.dims()[a.dims().len() - 2]);
    dims.push(b.dims()[a.dims().len() - 1]);
    // calculate value of output
    let mut output: Tensor<T> = Tensor::new(None, &dims).unwrap();

    let indices = dims.iter().map(|d| 0..*d).collect::<Vec<_>>();

    for coord in indices.iter().cloned().multi_cartesian_product() {
        let row = coord[0..coord.len() - 1]
            .iter()
            .map(|&d| d..(d + 1))
            .collect::<Vec<_>>();
        let mut col = coord[0..coord.len()]
            .iter()
            .map(|&d| d..(d + 1))
            .collect::<Vec<_>>();
        col[coord.len() - 2] = 0..b.dims()[coord.len() - 2];
        let prod = dot(&vec![&a.get_slice(&row[0..])?, &b.get_slice(&col[0..])?])?;
        output.set(&coord, prod[0].clone());
    }

    Ok(output)
}

/// Adds multiple tensors.
/// # Arguments
///
/// * `t` - Vector of tensors
/// # Examples
/// ```
/// use ezkl::tensor::Tensor;
/// use ezkl::tensor::ops::add;
/// let x = Tensor::<i32>::new(
///     Some(&[2, 1, 2, 1, 1, 1]),
///     &[2, 3],
/// ).unwrap();
/// let k = Tensor::<i32>::new(
///     Some(&[2, 3, 2, 1, 1, 1]),
///     &[2, 3],
/// ).unwrap();
/// let result = add(&vec![x, k]).unwrap();
/// let expected = Tensor::<i32>::new(Some(&[4, 4, 4, 2, 2, 2]), &[2, 3]).unwrap();
/// assert_eq!(result, expected);
/// ```
pub fn add<T: TensorType + Add<Output = T>>(t: &Vec<Tensor<T>>) -> Result<Tensor<T>, TensorError> {
    // determines if we're multiplying by a 1D const
    if t.len() == 2 && t[1].dims().len() == 1 && t[1].dims()[0] == 1 {
        return const_add(&t[0], t[1][0].clone());
    }
    for e in t.iter() {
        if t[0].dims() != e.dims() {
            return Err(TensorError::DimMismatch("add".to_string()));
        }
    }
    // calculate value of output
    let mut output: Tensor<T> = t[0].clone();

    for e in t[1..].iter() {
        for (i, e_i) in e.iter().enumerate() {
            output[i] = output[i].clone() + e_i.clone()
        }
    }

    Ok(output)
}

/// Elementwise adds a tensor with a const element.
/// # Arguments
///
/// * `a` - Tensor
/// * `b` - Single value
/// # Examples
/// ```
/// use ezkl::tensor::Tensor;
/// use ezkl::tensor::ops::const_add;
/// let x = Tensor::<i32>::new(
///     Some(&[2, 1, 2, 1, 1, 1]),
///     &[2, 3],
/// ).unwrap();
/// let k = 2;
/// let result = const_add(&x, k).unwrap();
/// let expected = Tensor::<i32>::new(Some(&[4, 3, 4, 3, 3, 3]), &[2, 3]).unwrap();
/// assert_eq!(result, expected);
/// ```
pub fn const_add<T: TensorType + Add<Output = T>>(
    a: &Tensor<T>,
    b: T,
) -> Result<Tensor<T>, TensorError> {
    // calculate value of output
    let mut output: Tensor<T> = a.clone();

    for i in 0..output.len() {
        output[i] = output[i].clone() + b.clone();
    }

    Ok(output)
}

/// Subtracts multiple tensors.
/// # Arguments
///
/// * `a` - Tensor
/// * `b` - Tensor
/// # Examples
/// ```
/// use ezkl::tensor::Tensor;
/// use ezkl::tensor::ops::sub;
/// let x = Tensor::<i32>::new(
///     Some(&[2, 1, 2, 1, 1, 1]),
///     &[2, 3],
/// ).unwrap();
/// let k = Tensor::<i32>::new(
///     Some(&[2, 3, 2, 1, 1, 1]),
///     &[2, 3],
/// ).unwrap();
/// let result = sub(&vec![x, k]).unwrap();
/// let expected = Tensor::<i32>::new(Some(&[0, -2, 0, 0, 0, 0]), &[2, 3]).unwrap();
/// assert_eq!(result, expected);
/// ```
pub fn sub<T: TensorType + Sub<Output = T>>(t: &Vec<Tensor<T>>) -> Result<Tensor<T>, TensorError> {
    // determines if we're multiplying by a 1D const
    if t.len() == 2 && t[1].dims().len() == 1 && t[1].dims()[0] == 1 {
        return const_sub(&t[0], t[1][0].clone());
    }

    for e in t.iter() {
        if t[0].dims() != e.dims() {
            return Err(TensorError::DimMismatch("sub".to_string()));
        }
    }
    // calculate value of output
    let mut output: Tensor<T> = t[0].clone();

    for e in t[1..].iter() {
        for (i, e_i) in e.iter().enumerate() {
            output[i] = output[i].clone() - e_i.clone()
        }
    }

    Ok(output)
}

/// Elementwise subtracts a tensor with a const element.
/// # Arguments
///
/// * `a` - Tensor
/// * `b` - Single value
/// # Examples
/// ```
/// use ezkl::tensor::Tensor;
/// use ezkl::tensor::ops::const_sub;
/// let x = Tensor::<i32>::new(
///     Some(&[2, 1, 2, 1, 1, 1]),
///     &[2, 3],
/// ).unwrap();
/// let k = 2;
/// let result = const_sub(&x, k).unwrap();
/// let expected = Tensor::<i32>::new(Some(&[0, -1, 0, -1, -1, -1]), &[2, 3]).unwrap();
/// assert_eq!(result, expected);
/// ```
pub fn const_sub<T: TensorType + Sub<Output = T>>(
    a: &Tensor<T>,
    b: T,
) -> Result<Tensor<T>, TensorError> {
    // calculate value of output
    let mut output: Tensor<T> = a.clone();

    for i in 0..output.len() {
        output[i] = output[i].clone() - b.clone();
    }

    Ok(output)
}

/// Elementwise multiplies two tensors.
/// # Arguments
///
/// * `a` - Tensor
/// * `b` - Tensor
/// # Examples
/// ```
/// use ezkl::tensor::Tensor;
/// use ezkl::tensor::ops::mult;
/// let x = Tensor::<i32>::new(
///     Some(&[2, 1, 2, 1, 1, 1]),
///     &[2, 3],
/// ).unwrap();
/// let k = Tensor::<i32>::new(
///     Some(&[2, 3, 2, 1, 1, 1]),
///     &[2, 3],
/// ).unwrap();
/// let result = mult(&vec![x, k]).unwrap();
/// let expected = Tensor::<i32>::new(Some(&[4, 3, 4, 1, 1, 1]), &[2, 3]).unwrap();
/// assert_eq!(result, expected);
/// ```
pub fn mult<T: TensorType + Mul<Output = T>>(t: &Vec<Tensor<T>>) -> Result<Tensor<T>, TensorError> {
    // determines if we're multiplying by a 1D const
    if t.len() == 2 && t[1].dims().len() == 1 && t[1].dims()[0] == 1 {
        return const_mult(&t[0], t[1][0].clone());
    }

    for e in t.iter() {
        if t[0].dims() != e.dims() {
            return Err(TensorError::DimMismatch("mult".to_string()));
        }
    }
    // calculate value of output
    let mut output: Tensor<T> = t[0].clone();

    for e in t[1..].iter() {
        for (i, e_i) in e.iter().enumerate() {
            output[i] = output[i].clone() * e_i.clone()
        }
    }

    Ok(output)
}

/// Elementwise divide a tensor with another tensor.
/// # Arguments
///
/// * `t` - Tensor
/// * `d` - Tensor
/// # Examples
/// ```
/// use ezkl::tensor::Tensor;
/// use ezkl::tensor::ops::div;
/// let x = Tensor::<i32>::new(
///     Some(&[4, 1, 4, 1, 1, 4]),
///     &[2, 3],
/// ).unwrap();
/// let y = Tensor::<i32>::new(
///     Some(&[2, 1, 2, 1, 1, 1]),
///     &[2, 3],
/// ).unwrap();
/// let result = div(x, y).unwrap();
/// let expected = Tensor::<i32>::new(Some(&[2, 1, 2, 1, 1, 4]), &[2, 3]).unwrap();
/// assert_eq!(result, expected);
/// ```
pub fn div<T: TensorType + Div<Output = T>>(
    t: Tensor<T>,
    d: Tensor<T>,
) -> Result<Tensor<T>, TensorError> {
    if t.dims() != d.dims() {
        return Err(TensorError::DimMismatch("div".to_string()));
    }
    // calculate value of output
    let mut output: Tensor<T> = t;

    for (i, d_i) in d.iter().enumerate() {
        output[i] = output[i].clone() / d_i.clone()
    }
    Ok(output)
}

/// Elementwise multiplies a tensor with a const element.
/// # Arguments
///
/// * `a` - Tensor
/// * `b` - Single value
/// # Examples
/// ```
/// use ezkl::tensor::Tensor;
/// use ezkl::tensor::ops::const_mult;
/// let x = Tensor::<i32>::new(
///     Some(&[2, 1, 2, 1, 1, 1]),
///     &[2, 3],
/// ).unwrap();
/// let k = 2;
/// let result = const_mult(&x, k).unwrap();
/// let expected = Tensor::<i32>::new(Some(&[4, 2, 4, 2, 2, 2]), &[2, 3]).unwrap();
/// assert_eq!(result, expected);
/// ```
pub fn const_mult<T: TensorType + Mul<Output = T>>(
    a: &Tensor<T>,
    b: T,
) -> Result<Tensor<T>, TensorError> {
    // calculate value of output
    let mut output: Tensor<T> = a.clone();

    for i in 0..output.len() {
        output[i] = output[i].clone() * b.clone();
    }

    Ok(output)
}

/// Rescale a tensor with a const integer (similar to const_mult).
/// # Arguments
///
/// * `a` - Tensor
/// * `b` - Single value
/// # Examples
/// ```
/// use ezkl::tensor::Tensor;
/// use ezkl::tensor::ops::rescale;
/// let x = Tensor::<i32>::new(
///     Some(&[2, 1, 2, 1, 1, 1]),
///     &[2, 3],
/// ).unwrap();
/// let k = 2;
/// let result = rescale(&x, k).unwrap();
/// let expected = Tensor::<i32>::new(Some(&[4, 2, 4, 2, 2, 2]), &[2, 3]).unwrap();
/// assert_eq!(result, expected);
/// ```
pub fn rescale<T: TensorType + Add<Output = T>>(
    a: &Tensor<T>,
    mult: usize,
) -> Result<Tensor<T>, TensorError> {
    // calculate value of output
    let mut output: Tensor<T> = a.clone();
    for (i, a_i) in a.iter().enumerate() {
        for _ in 1..mult {
            output[i] = output[i].clone() + a_i.clone();
        }
    }
    Ok(output)
}

/// Elementwise raise a tensor to the nth power.
/// # Arguments
///
/// * `a` - Tensor
/// * `b` - Single value
/// # Examples
/// ```
/// use ezkl::tensor::Tensor;
/// use ezkl::tensor::ops::pow;
/// let x = Tensor::<i32>::new(
///     Some(&[2, 15, 2, 1, 1, 0]),
///     &[2, 3],
/// ).unwrap();
/// let result = pow(&x, 3).unwrap();
/// let expected = Tensor::<i32>::new(Some(&[8, 3375, 8, 1, 1, 0]), &[2, 3]).unwrap();
/// assert_eq!(result, expected);
/// ```
pub fn pow<T: TensorType + Mul<Output = T>>(
    a: &Tensor<T>,
    pow: usize,
) -> Result<Tensor<T>, TensorError> {
    // calculate value of output
    let mut output: Tensor<T> = a.clone();
    for (i, a_i) in a.iter().enumerate() {
        for _ in 1..pow {
            output[i] = output[i].clone() * a_i.clone();
        }
    }
    Ok(output)
}

/// Sums a tensor.
/// # Arguments
///
/// * `a` - Tensor
/// * `b` - Single value
/// # Examples
/// ```
/// use ezkl::tensor::Tensor;
/// use ezkl::tensor::ops::sum;
/// let x = Tensor::<i32>::new(
///     Some(&[2, 15, 2, 1, 1, 0]),
///     &[2, 3],
/// ).unwrap();
/// let result = sum(&x).unwrap();
/// let expected = 21;
/// assert_eq!(result[0], expected);
/// ```
pub fn sum<T: TensorType + Add<Output = T>>(a: &Tensor<T>) -> Result<Tensor<T>, TensorError> {
    // calculate value of output
    let mut res = T::zero().unwrap();
    let _ = a.map(|a_i| res = res.clone() + a_i);
    Tensor::new(Some(&[res]), &[1])
}

/// Applies convolution over a 3D tensor of shape C x H x W (and adds a bias).
/// # Arguments
///
/// * `inputs` - A vector of tensors holding in order: input image, convolution kernel, convolution bias.
/// * `padding` - Tuple of padding values in x and y directions.
/// * `stride` - Tuple of stride values in x and y directions.
/// # Examples
/// ```
/// use ezkl::tensor::Tensor;
/// use ezkl::tensor::ops::convolution;
///
/// let x = Tensor::<i32>::new(
///     Some(&[5, 2, 3, 0, 4, -1, 3, 1, 6]),
///     &[1, 3, 3],
/// ).unwrap();
/// let k = Tensor::<i32>::new(
///     Some(&[5, 1, 1, 1]),
///     &[1, 1, 2, 2],
/// ).unwrap();
/// let b = Tensor::<i32>::new(
///     Some(&[0]),
///     &[1],
/// ).unwrap();
/// let result = convolution::<i32>(&vec![x, k, b], (0, 0), (1, 1)).unwrap();
/// let expected = Tensor::<i32>::new(Some(&[31, 16, 8, 26]), &[1, 2, 2]).unwrap();
/// assert_eq!(result, expected);
/// ```
pub fn convolution<T: TensorType + Mul<Output = T> + Add<Output = T>>(
    inputs: &Vec<Tensor<T>>,
    padding: (usize, usize),
    stride: (usize, usize),
) -> Result<Tensor<T>, TensorError> {
    let has_bias = inputs.len() == 3;
    let (image, kernel) = (inputs[0].clone(), inputs[1].clone());

    if (image.dims().len() != 3)
        || (kernel.dims().len() != 4)
        || (image.dims()[0] != kernel.dims()[1])
    {
        return Err(TensorError::DimMismatch("conv".to_string()));
    }

    if has_bias {
        let bias = inputs[2].clone();
        if (bias.dims().len() != 1) || (bias.dims()[0] != kernel.dims()[0]) {
            return Err(TensorError::DimMismatch("conv bias".to_string()));
        }
    }

    let image_dims = image.dims();
    let kernel_dims = kernel.dims();

    let (output_channels, input_channels, kernel_height, kernel_width) = (
        kernel_dims[0],
        kernel_dims[1],
        kernel_dims[2],
        kernel_dims[3],
    );

    let (image_height, image_width) = (image_dims[1], image_dims[2]);

    let padded_image = pad::<T>(&image, padding)?;

    let vert_slides = (image_height + 2 * padding.0 - kernel_height) / stride.0 + 1;
    let horz_slides = (image_width + 2 * padding.1 - kernel_width) / stride.1 + 1;

    // calculate value of output
    let mut output: Tensor<T> =
        Tensor::new(None, &[output_channels, vert_slides, horz_slides]).unwrap();

    for i in 0..output_channels {
        for j in 0..vert_slides {
            let rs = j * stride.0;
            for k in 0..horz_slides {
                let cs = k * stride.1;
                let mut res = dot(&vec![
                    &kernel.get_slice(&[i..i + 1])?.clone(),
                    &padded_image.get_slice(&[
                        0..input_channels,
                        rs..(rs + kernel_height),
                        cs..(cs + kernel_width),
                    ])?,
                ])?;

                if has_bias {
                    // increment result by the bias
                    res[0] = res[0].clone() + inputs[2][i].clone();
                }

                output.set(&[i, j, k], res[0].clone());
            }
        }
    }
    Ok(output)
}

/// Applies 2D sum pooling over a 3D tensor of shape C x H x W.
/// # Arguments
///
/// * `image` - Tensor.
/// * `padding` - Tuple of padding values in x and y directions.
/// * `stride` - Tuple of stride values in x and y directions.
/// * `pool_dims` - Tuple of pooling window size in x and y directions.
/// # Examples
/// ```
/// use ezkl::tensor::Tensor;
/// use ezkl::tensor::ops::sumpool;
/// use halo2_proofs::circuit::Value;
/// use halo2_proofs::plonk::Assigned;
/// use halo2curves::pasta::Fp as F;
///
/// let x = Tensor::<i32>::new(
///     Some(&[5, 2, 3, 0, 4, -1, 3, 1, 6]),
///     &[1, 3, 3],
/// ).unwrap();
/// let pooled = sumpool::<i32>(&x, (0, 0), (1, 1), (2, 2)).unwrap();
/// let expected: Tensor<i32> = Tensor::<i32>::new(Some(&[11, 8, 8, 10]), &[1, 2, 2]).unwrap();
/// assert_eq!(pooled, expected);
/// ```
pub fn sumpool<T: TensorType + Mul<Output = T> + Add<Output = T>>(
    image: &Tensor<T>,
    padding: (usize, usize),
    stride: (usize, usize),
    kernel_shape: (usize, usize),
) -> Result<Tensor<T>, TensorError> {
    if image.dims().len() != 3 {
        return Err(TensorError::DimMismatch("sumpool".to_string()));
    }
    let image_dims = image.dims();

    let (image_channels, image_height, image_width) = (image_dims[0], image_dims[1], image_dims[2]);

    let (output_channels, kernel_height, kernel_width) =
        (image_channels, kernel_shape.0, kernel_shape.1);

    let padded_image = pad::<T>(image, padding)?;

    let vert_slides = (image_height + 2 * padding.0 - kernel_height) / stride.0 + 1;
    let horz_slides = (image_width + 2 * padding.1 - kernel_width) / stride.1 + 1;

    // calculate value of output
    let mut output: Tensor<T> =
        Tensor::new(None, &[output_channels, vert_slides, horz_slides]).unwrap();

    for i in 0..output_channels {
        for j in 0..vert_slides {
            let rs = j * stride.0;
            for k in 0..horz_slides {
                let cs = k * stride.1;
                let thesum = sum(&padded_image.get_slice(&[
                    i..i + 1,
                    rs..(rs + kernel_height),
                    cs..(cs + kernel_width),
                ])?)?;
                output.set(&[i, j, k], thesum[0].clone());
            }
        }
    }
    Ok(output)
}

/// Applies 2D max pooling over a 3D tensor of shape C x H x W.
/// # Arguments
///
/// * `image` - Tensor.
/// * `padding` - Tuple of padding values in x and y directions.
/// * `stride` - Tuple of stride values in x and y directions.
/// * `pool_dims` - Tuple of pooling window size in x and y directions.
/// # Examples
/// ```
/// use ezkl::tensor::Tensor;
/// use ezkl::tensor::ops::max_pool2d;
/// use halo2_proofs::circuit::Value;
/// use halo2_proofs::plonk::Assigned;
/// use halo2curves::pasta::Fp as F;
///
/// let x = Tensor::<i32>::new(
///     Some(&[5, 2, 3, 0, 4, -1, 3, 1, 6]),
///     &[1, 3, 3],
/// ).unwrap();
/// let pooled = max_pool2d::<i32>(&x, (0, 0), (1, 1), (2, 2)).unwrap();
/// let expected: Tensor<i32> = Tensor::<i32>::new(Some(&[5, 4, 4, 6]), &[1, 2, 2]).unwrap();
/// assert_eq!(pooled, expected);
/// ```
pub fn max_pool2d<T: TensorType>(
    image: &Tensor<T>,
    padding: (usize, usize),
    stride: (usize, usize),
    pool_dims: (usize, usize),
) -> Result<Tensor<T>, TensorError> {
    if image.dims().len() != 3 {
        return Err(TensorError::DimMismatch("max_pool2d".to_string()));
    }
    let image_dims = image.dims();

    let input_channels = image_dims[0];
    let (image_height, image_width) = (image_dims[1], image_dims[2]);

    let padded_image = pad::<T>(image, padding)?;

    let horz_slides = (image_height + 2 * padding.0 - pool_dims.0) / stride.0 + 1;
    let vert_slides = (image_width + 2 * padding.1 - pool_dims.1) / stride.1 + 1;

    let mut output: Tensor<T> =
        Tensor::new(None, &[input_channels, horz_slides, vert_slides]).unwrap();

    let fmax = |acc: Option<T>, x: T| -> Option<T> {
        match (acc, x) {
            (None, x) => Some(x),
            (Some(a), x) => a.tmax(&x),
        }
    };

    for i in 0..input_channels {
        for j in 0..horz_slides {
            let rs = j * stride.0;
            for k in 0..vert_slides {
                let cs = k * stride.1;
                output.set(
                    &[i, j, k],
                    padded_image
                        .get_slice(&[i..(i + 1), rs..(rs + pool_dims.0), cs..(cs + pool_dims.1)])?
                        .into_iter()
                        .fold(None, fmax)
                        .unwrap(),
                );
            }
        }
    }
    Ok(output)
}

/// Dot product of two tensors.
/// # Arguments
///
/// * `inputs` - Vector of tensors of length 2.
/// # Examples
/// ```
/// use ezkl::tensor::Tensor;
/// use ezkl::tensor::ops::dot;
///
/// let x = Tensor::<i32>::new(
///     Some(&[5, 2, 3, 0, 4, -1, 3, 1, 6]),
///     &[1, 3, 3],
/// ).unwrap();
/// let y = Tensor::<i32>::new(
///     Some(&[5, 5, 10, -4, 2, -1, 2, 0, 1]),
///     &[1, 3, 3],
/// ).unwrap();
/// assert_eq!(dot(&vec![&x, &y]).unwrap()[0], 86);
/// ```
pub fn dot<T: TensorType + Mul<Output = T> + Add<Output = T>>(
    inputs: &Vec<&Tensor<T>>,
) -> Result<Tensor<T>, TensorError> {
    if (inputs.len() != 2) || (inputs[0].clone().len() != inputs[1].clone().len()) {
        return Err(TensorError::DimMismatch("dot".to_string()));
    }
    let (a, b): (Tensor<T>, Tensor<T>) = (inputs[0].clone(), inputs[1].clone());
    let res = a
        .iter()
        .zip(b)
        .fold(T::zero().unwrap(), |acc, (k, i)| acc + k.clone() * i);
    Tensor::new(Some(&[res]), &[1])
}

/// Pads a 3D tensor of shape `C x H x W` to a tensor of shape `C x (H + 2xPADDING) x (W + 2xPADDING)` using 0 values.
/// # Arguments
///
/// * `image` - Tensor.
/// * `padding` - Tuple of padding values in x and y directions.
/// # Examples
/// ```
/// use ezkl::tensor::Tensor;
/// use ezkl::tensor::ops::pad;
///
/// let x = Tensor::<i32>::new(
///     Some(&[5, 2, 3, 0, 4, -1, 3, 1, 6]),
///     &[1, 3, 3],
/// ).unwrap();
/// let result = pad::<i32>(&x, (1, 1)).unwrap();
/// let expected = Tensor::<i32>::new(
///     Some(&[0, 0, 0, 0, 0, 0, 5, 2, 3, 0, 0, 0, 4, -1, 0, 0, 3, 1, 6, 0, 0, 0, 0, 0, 0]),
///     &[1, 5, 5],
/// ).unwrap();
/// assert_eq!(result, expected);
/// ```
pub fn pad<T: TensorType>(
    image: &Tensor<T>,
    padding: (usize, usize),
) -> Result<Tensor<T>, TensorError> {
    if image.dims().len() != 3 {
        return Err(TensorError::DimMismatch("pad".to_string()));
    }
    let (channels, height, width) = (image.dims()[0], image.dims()[1], image.dims()[2]);
    let padded_height = height + 2 * padding.0;
    let padded_width = width + 2 * padding.1;

    let mut output = Tensor::<T>::new(None, &[channels, padded_height, padded_width]).unwrap();

    for channel in 0..channels {
        for row in 0..height {
            for col in 0..width {
                output.set(
                    &[channel, row + padding.0, col + padding.1],
                    image.get(&[channel, row, col]).clone(),
                );
            }
        }
    }

    output.reshape(&[channels, padded_height, padded_width]);
    Ok(output)
}

// ---------------------------------------------------------------------------------------------------------
// -- nonlinear Functions ---------------------------------------------------------------------------------
// ---------------------------------------------------------------------------------------------------------
// ---------------------------------------------------------------------------------------------------------
// ---------------------------------------------------------------------------------------------------------
// ---------------------------------------------------------------------------------------------------------

/// Activation functions
pub mod nonlinearities {
    use super::*;
    /// Elementwise applies sigmoid to a tensor of integers.
    /// # Arguments
    ///
    /// * `a` - Tensor
    /// * `scale_input` - Single value
    /// * `scale_output` - Single value
    /// # Examples
    /// ```
    /// use ezkl::tensor::Tensor;
    /// use ezkl::tensor::ops::nonlinearities::sigmoid;
    /// let x = Tensor::<i32>::new(
    ///     Some(&[2, 15, 2, 1, 1, 0]),
    ///     &[2, 3],
    /// ).unwrap();
    /// let result = sigmoid(&x, 1, 1);
    /// let expected = Tensor::<i32>::new(Some(&[1, 1, 1, 1, 1, 1]), &[2, 3]).unwrap();
    /// assert_eq!(result, expected);
    /// ```
    pub fn sigmoid(a: &Tensor<i32>, scale_input: usize, scale_output: usize) -> Tensor<i32> {
        // calculate value of output
        let mut output: Tensor<i32> = a.clone();

        for (i, a_i) in a.iter().enumerate() {
            let kix = (*a_i as f32) / (scale_input as f32);
            let fout = (scale_output as f32) / (1.0 + (-kix).exp());
            let rounded = fout.round();
            output[i] = rounded as i32;
        }
        output
    }

    /// Elementwise applies sigmoid to a tensor of integers.
    /// # Arguments
    ///
    /// * `a` - Tensor
    /// * `scale_input` - Single value
    /// * `scale_output` - Single value
    /// # Examples
    /// ```
    /// use ezkl::tensor::Tensor;
    /// use ezkl::tensor::ops::nonlinearities::sqrt;
    /// let x = Tensor::<i32>::new(
    ///     Some(&[4, 25, 8, 1, 1, 0]),
    ///     &[2, 3],
    /// ).unwrap();
    /// let result = sqrt(&x, 1, 1);
    /// let expected = Tensor::<i32>::new(Some(&[2, 5, 3, 1, 1, 0]), &[2, 3]).unwrap();
    /// assert_eq!(result, expected);
    /// ```
    pub fn sqrt(a: &Tensor<i32>, scale_input: usize, scale_output: usize) -> Tensor<i32> {
        // calculate value of output
        let mut output: Tensor<i32> = a.clone();

        for (i, a_i) in a.iter().enumerate() {
            let kix = (*a_i as f32) / (scale_input as f32);
            let fout = (scale_output as f32) * kix.sqrt();
            let rounded = fout.round();
            output[i] = rounded as i32;
        }
        output
    }

    /// Elementwise applies leaky relu to a tensor of integers.
    /// # Arguments
    ///
    /// * `a` - Tensor
    /// * `scale` - Single value
    /// * `slope` - Single value
    /// # Examples
    /// ```
    /// use ezkl::tensor::Tensor;
    /// use ezkl::tensor::ops::nonlinearities::leakyrelu;
    /// let x = Tensor::<i32>::new(
    ///     Some(&[2, 15, 2, 1, 1, -5]),
    ///     &[2, 3],
    /// ).unwrap();
    /// let result = leakyrelu(&x, 1, 0.1);
    /// let expected = Tensor::<i32>::new(Some(&[2, 15, 2, 1, 1, -1]), &[2, 3]).unwrap();
    /// assert_eq!(result, expected);
    /// ```
    pub fn leakyrelu(a: &Tensor<i32>, scale: usize, slope: f32) -> Tensor<i32> {
        // calculate value of output
        let mut output: Tensor<i32> = a.clone();

        for (i, a_i) in a.iter().enumerate() {
            output[i] = if a_i < &0 {
                let d_inv_x = (slope) * (*a_i as f32) / (scale as f32);
                d_inv_x.round() as i32
            } else {
                let d_inv_x = (*a_i as f32) / (scale as f32);
                d_inv_x.round() as i32
            };
        }
        output
    }

    /// Elementwise applies prelu to a tensor of integers.
    /// # Arguments
    ///
    /// * `a` - Tensor
    /// * `scale` - Single value
    /// * `slopes` - Array of values
    /// # Examples
    /// ```
    /// use ezkl::tensor::Tensor;
    /// use ezkl::tensor::ops::nonlinearities::prelu;
    /// let x = Tensor::<i32>::new(
    ///     Some(&[-10, 15, 2, 1, 1, -5]),
    ///     &[2, 3],
    /// ).unwrap();
    /// let result = prelu(&x, 1, &[0.1, 25.0]);
    /// let expected = Tensor::<i32>::new(Some(&[-1, 15, 2, 1, 1, -125]), &[2, 3]).unwrap();
    /// assert_eq!(result, expected);
    /// ```
    pub fn prelu(a: &Tensor<i32>, scale: usize, slopes: &[f32]) -> Tensor<i32> {
        if slopes.len() == 1 {
            return leakyrelu(a, scale, slopes[0]);
        } else {
            // assert number of slopes is equal to number of channels
            assert_eq!(slopes.len(), a.dims()[0])
        }
        // calculate value of output
        let mut output: Tensor<i32> = a.clone();

        for (i, a_i) in a.iter().enumerate() {
            output[i] = if a_i < &0 {
                let slope_i: f32 = slopes[i / (a.dims()[1..].iter().product::<usize>())];
                let d_inv_x = (slope_i) * (*a_i as f32) / (scale as f32);
                d_inv_x.round() as i32
            } else {
                let d_inv_x = (*a_i as f32) / (scale as f32);
                d_inv_x.round() as i32
            };
        }
        output
    }

    /// Elementwise divides a tensor with a const integer element.
    /// # Arguments
    ///
    /// * `a` - Tensor
    /// * `b` - Single value
    /// # Examples
    /// ```
    /// use ezkl::tensor::Tensor;
    /// use ezkl::tensor::ops::nonlinearities::const_div;
    /// let x = Tensor::<i32>::new(
    ///     Some(&[2, 1, 2, 7, 1, 1]),
    ///     &[2, 3],
    /// ).unwrap();
    /// let k = 2;
    /// let result = const_div(&x, k);
    /// let expected = Tensor::<i32>::new(Some(&[1, 1, 1, 4, 1, 1]), &[2, 3]).unwrap();
    /// assert_eq!(result, expected);
    /// ```
    pub fn const_div(a: &Tensor<i32>, scale: i32) -> Tensor<i32> {
        // calculate value of output
        // calculate value of output
        let mut output: Tensor<i32> = a.clone();

        for (i, a_i) in a.iter().enumerate() {
            let d_inv_x = (*a_i as f32) / (scale as f32);
            output[i] = d_inv_x.round() as i32;
        }
        output
    }
}
