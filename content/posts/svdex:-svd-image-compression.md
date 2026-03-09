---
title: "svdex: svd image compression"
description: "exploring image compression through singular value decomposition"
date: 2026-03-09
tags: [rust, math]
draft: true
---

i wanted to understand how linear algebra compresses images. not the "read a wikipedia article" kind of understanding - the "build it from scratch and watch it work" kind. so i wrote [svdex](https://github.com/aidantrabs/svdex), a little rust cli that compresses images using singular value decomposition.

here's what i learned along the way.

## the big idea

every matrix can be broken into three pieces. this is svd:

$$A = U \Sigma V^T$$

$U$ and $V^T$ are rotation matrices. $\Sigma$ is a diagonal matrix of "singular values" - numbers that tell you how important each component is. they come sorted from biggest to smallest: $\sigma_1 \geq \sigma_2 \geq \dots \geq \sigma_r > 0$.

the insight that makes compression possible: the first few singular values are usually *way* bigger than the rest. most of the information lives in a small number of components.

so what if we just... kept the top $k$ and threw the rest away?

$$A_k = \sum_{i=1}^{k} \sigma_i \mathbf{u}_i \mathbf{v}_i^T$$

turns out this is provably optimal. the eckart-young theorem says this rank-$k$ approximation is the best you can do - no other method using $k$ components will get you closer to the original. that's not a vague claim, it's a mathematical guarantee.

i got a lot of my initial intuition from [zerobone's post on svd image compression](https://zerobone.net/blog/cs/svd-image-compression/), which does a great job of connecting the math to what's actually happening with pixels.

## so how does this compress an image?

an image is just three matrices stacked on top of each other - one for red, one for green, one for blue. each entry is a pixel value from 0 to 255.

the plan is simple:

1. pull apart the rgb channels
2. run svd on each one
3. keep only the top $k$ singular values
4. put it back together

in code, the truncation step looks like this:

```rust
pub fn low_rank_approx(svd: &SvdResult, k: usize) -> Array2<f64> {
    let k = k.min(svd.s.len());

    let u_k = svd.u.slice(s![.., ..k]).to_owned();
    let s_k = &svd.s.slice(s![..k]);
    let vt_k = svd.vt.slice(s![..k, ..]).to_owned();

    let u_scaled = &u_k * s_k;
    u_scaled.dot(&vt_k)
}
```

three slices and a dot product. that's the whole thing.

one gotcha - the reconstructed values can land outside $[0, 255]$. if you don't clamp them before saving, you get weird wrapping artifacts. learned that one the hard way.

## how much space do we actually save?

original storage: $3 \cdot h \cdot w$ values (three full matrices).

compressed storage per channel: $h \times k$ for the truncated $U$, $k$ for the singular values, $k \times w$ for the truncated $V^T$. so:

$$\text{ratio} = \frac{3hw}{3k(h + 1 + w)}$$

for a $1200 \times 797$ image at rank 50, that's about $9.6\times$ compression. not bad for some matrix math.

## the numbers

i ran experiments across a bunch of ranks to see what actually happens:

| rank | ratio | mse | psnr |
|------|-------|-----|------|
| 1 | 478.68x | 2937.52 | 13.45 dB |
| 5 | 95.74x | 1265.33 | 17.11 dB |
| 10 | 47.87x | 878.57 | 18.69 dB |
| 20 | 23.93x | 620.23 | 20.21 dB |
| 50 | 9.57x | 361.29 | 22.55 dB |
| 100 | 4.79x | 212.83 | 24.85 dB |
| 200 | 2.39x | 92.52 | 28.47 dB |

(mse is mean squared error - lower is better. psnr is peak signal-to-noise ratio in decibels - higher is better.)

$$\text{MSE} = \frac{1}{N} \sum_{i} (x_i - \hat{x}_i)^2 \qquad \text{PSNR} = 10 \cdot \log_{10}\left(\frac{255^2}{\text{MSE}}\right)$$

some things jumped out at me.

**the first few components do most of the heavy lifting.** going from rank 1 to rank 20 cuts the error by almost $5\times$. going from rank 100 to rank 200 only halves it. the early gains are massive, then you hit diminishing returns fast.

**there's no magic number.** i kept expecting to find some rank where the image suddenly "clicks" into looking good. that doesn't happen. quality improves smoothly - you just pick where on the curve you're comfortable.

**below 20 dB it looks rough.** around 25 dB it starts looking fine for most purposes. the rank 50-100 range is the sweet spot for this image.

## the decay curve tells the whole story

svdex plots the singular values for all three channels. the shape is always the same - a steep initial drop, then a long flat tail.

the red channel's first singular value was ~79,000. by the 10th, it dropped to ~7,400. blue started highest at ~128,000 (lots of sky in the test image). by the time you're past the first hundred or so values, everything is close to zero.

this decay is *the entire reason svd compression works*. if singular values were spread out evenly, throwing any of them away would hurt equally and compression would be pointless. the steep drop means most of them barely matter.

## one implementation detail that matters

when running experiments across multiple ranks, compute svd once and reuse it:

```rust
pub fn compress_with_svds(svds: &[SvdResult; 3], k: usize) -> [Array2<f64>; 3] {
    [
        low_rank_approx(&svds[0], k),
        low_rank_approx(&svds[1], k),
        low_rank_approx(&svds[2], k),
    ]
}
```

the svd factorization is $O(\min(m, n) \cdot mn)$ - that's the expensive part. truncation is just slicing arrays. computing svd nine times instead of three because you forgot to cache it is a mistake you only make once (i made it once).

## what stuck with me

the eckart-young theorem went from "cool abstract result" to something i can see. you look at a rank-50 compressed image and know - mathematically, provably - that no other 50-component approximation could look better. that's wild.

and the singular value decay curve is the single most informative thing about any matrix you're trying to compress. everything about the quality-size tradeoff is encoded in that shape.

the source code is at [github.com/aidantrabs/svdex](https://github.com/aidantrabs/svdex).
