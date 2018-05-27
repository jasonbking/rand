// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// https://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Functions for randomly accessing and sampling sequences.
//! 
//! TODO: module doc

#[cfg(feature="alloc")] use core::ops::Index;

#[cfg(feature="std")] use std::vec;
#[cfg(all(feature="alloc", not(feature="std")))] use alloc::{vec, Vec};
// BTreeMap is not as fast in tests, but better than nothing.
#[cfg(feature="std")] use std::collections::HashMap;
#[cfg(all(feature="alloc", not(feature="std")))] use alloc::btree_map::BTreeMap;


use super::Rng;

/// Extension trait on slices, providing random mutation and sampling methods.
/// 
/// An implementation is provided for slices. This may also be implementable for
/// other types.
pub trait SliceRandom {
    /// The element type.
    type Item;

    /// Returns a reference to one random element of the slice, or `None` if the
    /// slice is empty.
    /// 
    /// Depending on the implementation, complexity is expected to be `O(1)`.
    ///
    /// # Example
    ///
    /// ```
    /// use rand::thread_rng;
    /// use rand::seq::SliceRandom;
    ///
    /// let choices = [1, 2, 4, 8, 16, 32];
    /// let mut rng = thread_rng();
    /// println!("{:?}", choices.choose(&mut rng));
    /// assert_eq!(choices[..0].choose(&mut rng), None);
    /// ```
    fn choose<R>(&self, rng: &mut R) -> Option<&Self::Item>
        where R: Rng + ?Sized;

    /// Returns a mutable reference to one random element of the slice, or
    /// `None` if the slice is empty.
    /// 
    /// Depending on the implementation, complexity is expected to be `O(1)`.
    fn choose_mut<R>(&mut self, rng: &mut R) -> Option<&mut Self::Item>
        where R: Rng + ?Sized;

    /// Produces an iterator that chooses `amount` elements from the slice at
    /// random without repeating any.
    ///
    /// In case this API is not sufficiently flexible, use `sample_indices` then
    /// apply the indices to the slice.
    /// 
    /// Although the elements are selected randomly, the order of returned
    /// elements is neither stable nor fully random. If random ordering is
    /// desired, either use `partial_shuffle` or use this method and shuffle
    /// the result. If stable order is desired, use `sample_indices`, sort the
    /// result, then apply to the slice.
    /// 
    /// Complexity is expected to be the same as `sample_indices`.
    /// 
    /// # Example
    /// ```
    /// use rand::seq::SliceRandom;
    /// 
    /// let mut rng = &mut rand::thread_rng();
    /// let sample = "Hello, audience!".as_bytes();
    /// 
    /// // collect the results into a vector:
    /// let v: Vec<u8> = sample.choose_multiple(&mut rng, 3).cloned().collect();
    /// 
    /// // store in a buffer:
    /// let mut buf = [0u8; 5];
    /// for (b, slot) in sample.choose_multiple(&mut rng, buf.len()).zip(buf.iter_mut()) {
    ///     *slot = *b;
    /// }
    /// ```
    #[cfg(feature = "alloc")]
    fn choose_multiple<R>(&self, rng: &mut R, amount: usize) -> SliceChooseIter<Self, Self::Item>
        where R: Rng + ?Sized;

    /// Shuffle a mutable slice in place.
    /// 
    /// Depending on the implementation, complexity is expected to be `O(1)`.
    ///
    /// # Example
    ///
    /// ```
    /// use rand::thread_rng;
    /// use rand::seq::SliceRandom;
    ///
    /// let mut rng = thread_rng();
    /// let mut y = [1, 2, 3, 4, 5];
    /// println!("Unshuffled: {:?}", y);
    /// y.shuffle(&mut rng);
    /// println!("Shuffled:   {:?}", y);
    /// ```
    fn shuffle<R>(&mut self, rng: &mut R) where R: Rng + ?Sized;

    /// Shuffle a slice in place, but exit early.
    ///
    /// Returns two mutable slices from the source slice. The first contains
    /// `amount` elements randomly permuted. The second has the remaining
    /// elements that are not fully shuffled.
    ///
    /// This is an efficient method to select `amount` elements at random from
    /// the slice, provided the slice may be mutated.
    ///
    /// If you only need to choose elements randomly and `amount > self.len()/2`
    /// then you may improve performance by taking
    /// `amount = values.len() - amount` and using only the second slice.
    ///
    /// If `amount` is greater than the number of elements in the slice, this
    /// will perform a full shuffle.
    ///
    /// Complexity is expected to be `O(m)` where `m = amount`.
    fn partial_shuffle<R>(&mut self, rng: &mut R, amount: usize)
        -> (&mut [Self::Item], &mut [Self::Item]) where R: Rng + ?Sized;
}

/// Extension trait on iterators, providing random sampling methods.
pub trait IteratorRandom: Iterator + Sized {
    /// Choose one element at random from the iterator.
    ///
    /// Returns `None` if and only if the iterator is empty.
    /// 
    /// Complexity is `O(n)`, where `n` is the length of the iterator.
    /// This likely consumes multiple random numbers, but the exact number
    /// is unspecified.
    fn choose<R>(mut self, rng: &mut R) -> Option<Self::Item>
        where R: Rng + ?Sized
    {
        if let Some(elem) = self.next() {
            let mut result = elem;
            
            // Continue until the iterator is exhausted
            for (i, elem) in self.enumerate() {
                let denom = (i + 2) as f64; // accurate to 2^53 elements
                if rng.gen_bool(1.0 / denom) {
                    result = elem;
                }
            }
            
            Some(result)
        } else {
            None
        }
    }

    /// Collects `amount` values at random from the iterator into a supplied
    /// buffer.
    /// 
    /// Although the elements are selected randomly, the order of elements in
    /// the buffer is neither stable nor fully random. If random ordering is
    /// desired, shuffle the result.
    /// 
    /// Returns the number of elements added to the buffer. This equals `amount`
    /// unless the iterator contains insufficient elements, in which case this
    /// equals the number of elements available.
    /// 
    /// Complexity is `O(n)` where `n` is the length of the iterator.
    fn choose_multiple_fill<R>(mut self, rng: &mut R, buf: &mut [Self::Item])
        -> usize where R: Rng + ?Sized
    {
        let amount = buf.len();
        let mut len = 0;
        while len < amount {
            if let Some(elem) = self.next() {
                buf[len] = elem;
                len += 1;
            } else {
                // Iterator exhausted; stop early
                return len;
            }
        }

        // Continue, since the iterator was not exhausted
        for (i, elem) in self.enumerate() {
            let k = rng.gen_range(0, i + 1 + amount);
            if let Some(slot) = buf.get_mut(k) {
                *slot = elem;
            }
        }
        len
    }

    /// Collects `amount` values at random from the iterator into a vector.
    ///
    /// This is equivalent to `choose_multiple_fill` except for the result type.
    ///
    /// Although the elements are selected randomly, the order of elements in
    /// the buffer is neither stable nor fully random. If random ordering is
    /// desired, shuffle the result.
    /// 
    /// The length of the returned vector equals `amount` unless the iterator
    /// contains insufficient elements, in which case it equals the number of
    /// elements available.
    /// 
    /// Complexity is `O(n)` where `n` is the length of the iterator.
    #[cfg(feature = "alloc")]
    fn choose_multiple<R>(mut self, rng: &mut R, amount: usize) -> Vec<Self::Item>
        where R: Rng + ?Sized
    {
        let mut reservoir = Vec::with_capacity(amount);
        reservoir.extend(self.by_ref().take(amount));

        // Continue unless the iterator was exhausted
        //
        // note: this prevents iterators that "restart" from causing problems.
        // If the iterator stops once, then so do we.
        if reservoir.len() == amount {
            for (i, elem) in self.enumerate() {
                let k = rng.gen_range(0, i + 1 + amount);
                if let Some(slot) = reservoir.get_mut(k) {
                    *slot = elem;
                }
            }
        } else {
            // Don't hang onto extra memory. There is a corner case where
            // `amount` was much less than `self.len()`.
            reservoir.shrink_to_fit();
        }
        reservoir
    }
}


impl<T> SliceRandom for [T] {
    type Item = T;

    fn choose<R>(&self, rng: &mut R) -> Option<&Self::Item>
        where R: Rng + ?Sized
    {
        if self.is_empty() {
            None
        } else {
            Some(&self[rng.gen_range(0, self.len())])
        }
    }

    fn choose_mut<R>(&mut self, rng: &mut R) -> Option<&mut Self::Item>
        where R: Rng + ?Sized
    {
        if self.is_empty() {
            None
        } else {
            let len = self.len();
            Some(&mut self[rng.gen_range(0, len)])
        }
    }

    #[cfg(feature = "alloc")]
    fn choose_multiple<R>(&self, rng: &mut R, amount: usize) -> SliceChooseIter<Self, Self::Item>
        where R: Rng + ?Sized
    {
        let amount = ::core::cmp::min(amount, self.len());
        SliceChooseIter {
            slice: self,
            _phantom: Default::default(),
            indices: sample_indices(rng, self.len(), amount).into_iter(),
        }
    }

    fn shuffle<R>(&mut self, rng: &mut R) where R: Rng + ?Sized
    {
        for i in (1..self.len()).rev() {
            // invariant: elements with index > i have been locked in place.
            self.swap(i, rng.gen_range(0, i + 1));
        }
    }

    fn partial_shuffle<R>(&mut self, rng: &mut R, amount: usize)
        -> (&mut [Self::Item], &mut [Self::Item]) where R: Rng + ?Sized
    {
        unimplemented!()
    }
}

impl<I> IteratorRandom for I where I: Iterator + Sized {}


/// Iterator over multiple choices, as returned by [`SliceRandom::choose_multiple](
/// trait.SliceRandom.html#method.choose_multiple).
#[cfg(feature = "alloc")]
#[derive(Debug)]
pub struct SliceChooseIter<'a, S: ?Sized + 'a, T: 'a> {
    slice: &'a S,
    _phantom: ::core::marker::PhantomData<T>,
    indices: vec::IntoIter<usize>,
}

#[cfg(feature = "alloc")]
impl<'a, S: Index<usize, Output = T> + ?Sized + 'a, T: 'a> Iterator for SliceChooseIter<'a, S, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        // TODO: investigate using SliceIndex::get_unchecked when stable
        self.indices.next().map(|i| &(*self.slice)[i])
    }
    
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.indices.len(), Some(self.indices.len()))
    }
}

#[cfg(feature = "alloc")]
impl<'a, S: Index<usize, Output = T> + ?Sized + 'a, T: 'a> ExactSizeIterator
    for SliceChooseIter<'a, S, T>
{
    fn len(&self) -> usize {
        self.indices.len()
    }
}


/// Randomly sample `amount` elements from a finite iterator.
///
/// Deprecated: use [`IteratorRandom::choose_multiple`] instead.
/// 
/// [`IteratorRandom::choose_multiple`]: trait.IteratorRandom.html#method.choose_multiple
#[cfg(feature = "alloc")]
#[deprecated(since="0.6.0", note="use IteratorRandom::choose_multiple instead")]
pub fn sample_iter<T, I, R>(rng: &mut R, iterable: I, amount: usize) -> Result<Vec<T>, Vec<T>>
    where I: IntoIterator<Item=T>,
          R: Rng + ?Sized,
{
    use seq::IteratorRandom;
    let iter = iterable.into_iter();
    let result = iter.choose_multiple(rng, amount);
    if result.len() == amount {
        Ok(result)
    } else {
        Err(result)
    }
}

/// Randomly sample exactly `amount` values from `slice`.
///
/// The values are non-repeating and in random order.
///
/// This implementation uses `O(amount)` time and memory.
///
/// Panics if `amount > slice.len()`
///
/// Deprecated: use [`SliceRandom::choose_multiple`] instead.
/// 
/// [`SliceRandom::choose_multiple`]: trait.SliceRandom.html#method.choose_multiple
#[cfg(feature = "alloc")]
#[deprecated(since="0.6.0", note="use SliceRandom::choose_multiple instead")]
pub fn sample_slice<R, T>(rng: &mut R, slice: &[T], amount: usize) -> Vec<T>
    where R: Rng + ?Sized,
          T: Clone
{
    let indices = sample_indices(rng, slice.len(), amount);

    let mut out = Vec::with_capacity(amount);
    out.extend(indices.iter().map(|i| slice[*i].clone()));
    out
}

/// Randomly sample exactly `amount` references from `slice`.
///
/// The references are non-repeating and in random order.
///
/// This implementation uses `O(amount)` time and memory.
///
/// Panics if `amount > slice.len()`
///
/// Deprecated: use [`SliceRandom::choose_multiple`] instead.
/// 
/// [`SliceRandom::choose_multiple`]: trait.SliceRandom.html#method.choose_multiple
#[cfg(feature = "alloc")]
#[deprecated(since="0.6.0", note="use SliceRandom::choose_multiple instead")]
pub fn sample_slice_ref<'a, R, T>(rng: &mut R, slice: &'a [T], amount: usize) -> Vec<&'a T>
    where R: Rng + ?Sized
{
    let indices = sample_indices(rng, slice.len(), amount);

    let mut out = Vec::with_capacity(amount);
    out.extend(indices.iter().map(|i| &slice[*i]));
    out
}

/// Randomly sample exactly `amount` indices from `0..length`.
///
/// The values are non-repeating and in random order.
///
/// This implementation uses `O(amount)` time and memory.
///
/// This method is used internally by the slice sampling methods, but it can sometimes be useful to
/// have the indices themselves so this is provided as an alternative.
///
/// Panics if `amount > length`
#[cfg(feature = "alloc")]
pub fn sample_indices<R>(rng: &mut R, length: usize, amount: usize) -> Vec<usize>
    where R: Rng + ?Sized,
{
    if amount > length {
        panic!("`amount` must be less than or equal to `slice.len()`");
    }

    // We are going to have to allocate at least `amount` for the output no matter what. However,
    // if we use the `cached` version we will have to allocate `amount` as a HashMap as well since
    // it inserts an element for every loop.
    //
    // Therefore, if `amount >= length / 2` then inplace will be both faster and use less memory.
    // In fact, benchmarks show the inplace version is faster for length up to about 20 times
    // faster than amount.
    //
    // TODO: there is probably even more fine-tuning that can be done here since
    // `HashMap::with_capacity(amount)` probably allocates more than `amount` in practice,
    // and a trade off could probably be made between memory/cpu, since hashmap operations
    // are slower than array index swapping.
    if amount >= length / 20 {
        sample_indices_inplace(rng, length, amount)
    } else {
        sample_indices_cache(rng, length, amount)
    }
}

/// Sample an amount of indices using an inplace partial fisher yates method.
///
/// This allocates the entire `length` of indices and randomizes only the first `amount`.
/// It then truncates to `amount` and returns.
///
/// This is better than using a `HashMap` "cache" when `amount >= length / 2`
/// since it does not require allocating an extra cache and is much faster.
#[cfg(feature = "alloc")]
fn sample_indices_inplace<R>(rng: &mut R, length: usize, amount: usize) -> Vec<usize>
    where R: Rng + ?Sized,
{
    debug_assert!(amount <= length);
    let mut indices: Vec<usize> = Vec::with_capacity(length);
    indices.extend(0..length);
    for i in 0..amount {
        let j: usize = rng.gen_range(i, length);
        indices.swap(i, j);
    }
    indices.truncate(amount);
    debug_assert_eq!(indices.len(), amount);
    indices
}


/// This method performs a partial fisher-yates on a range of indices using a
/// `HashMap` as a cache to record potential collisions.
///
/// The cache avoids allocating the entire `length` of values. This is especially useful when
/// `amount <<< length`, i.e. select 3 non-repeating from `1_000_000`
#[cfg(feature = "alloc")]
fn sample_indices_cache<R>(
    rng: &mut R,
    length: usize,
    amount: usize,
) -> Vec<usize>
    where R: Rng + ?Sized,
{
    debug_assert!(amount <= length);
    #[cfg(feature="std")] let mut cache = HashMap::with_capacity(amount);
    #[cfg(not(feature="std"))] let mut cache = BTreeMap::new();
    let mut out = Vec::with_capacity(amount);
    for i in 0..amount {
        let j: usize = rng.gen_range(i, length);

        // equiv: let tmp = slice[i];
        let tmp = match cache.get(&i) {
            Some(e) => *e,
            None => i,
        };

        // equiv: slice[i] = slice[j];
        let x = match cache.get(&j) {
            Some(x) => *x,
            None => j,
        };

        // equiv: slice[j] = tmp;
        cache.insert(j, tmp);

        // note that in the inplace version, slice[i] is automatically "returned" value
        out.push(x);
    }
    debug_assert_eq!(out.len(), amount);
    out
}

#[cfg(test)]
mod test {
    use super::*;
    use Rng;
    #[cfg(feature = "alloc")] use SeedableRng;
    #[cfg(feature = "alloc")] use prng::XorShiftRng;
    #[cfg(all(feature="alloc", not(feature="std")))]
    use alloc::Vec;

    #[test]
    fn test_choose() {
        let mut r = ::test::rng(107);
        assert_eq!([1, 1, 1].choose(&mut r), Some(&1));
        
        let mut v = [2];
        v.choose_mut(&mut r).map(|x| *x = 5);
        assert_eq!(v[0], 5);

        let v = [3, 3, 3, 3];
        assert_eq!(v.iter().choose(&mut r), Some(&3));

        let v: &[isize] = &[];
        assert_eq!(v.choose(&mut r), None);
    }

    #[test]
    fn test_shuffle() {
        let mut r = ::test::rng(108);
        let empty: &mut [isize] = &mut [];
        empty.shuffle(&mut r);
        let mut one = [1];
        one.shuffle(&mut r);
        let b: &[_] = &[1];
        assert_eq!(one, b);

        let mut two = [1, 2];
        two.shuffle(&mut r);
        assert!(two == [1, 2] || two == [2, 1]);

        let mut x = [1, 1, 1];
        x.shuffle(&mut r);
        let b: &[_] = &[1, 1, 1];
        assert_eq!(x, b);
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn test_sample_iter() {
        let min_val = 1;
        let max_val = 100;

        let mut r = ::test::rng(401);
        let vals = (min_val..max_val).collect::<Vec<i32>>();
        let small_sample = vals.iter().choose_multiple(&mut r, 5);
        let large_sample = vals.iter().choose_multiple(&mut r, vals.len() + 5);

        assert_eq!(small_sample.len(), 5);
        assert_eq!(large_sample.len(), vals.len());
        // no randomization happens when amount >= len
        assert_eq!(large_sample, vals.iter().collect::<Vec<_>>());

        assert!(small_sample.iter().all(|e| {
            **e >= min_val && **e <= max_val
        }));
    }
    
    #[test]
    #[cfg(feature = "alloc")]
    #[allow(deprecated)]
    fn test_sample_slice_boundaries() {
        let empty: &[u8] = &[];

        let mut r = ::test::rng(402);

        // sample 0 items
        assert_eq!(&sample_slice(&mut r, empty, 0)[..], [0u8; 0]);
        assert_eq!(&sample_slice(&mut r, &[42, 2, 42], 0)[..], [0u8; 0]);

        // sample 1 item
        assert_eq!(&sample_slice(&mut r, &[42], 1)[..], [42]);
        let v = sample_slice(&mut r, &[1, 42], 1)[0];
        assert!(v == 1 || v == 42);

        // sample "all" the items
        let v = sample_slice(&mut r, &[42, 133], 2);
        assert!(&v[..] == [42, 133] || v[..] == [133, 42]);

        assert_eq!(&sample_indices_inplace(&mut r, 0, 0)[..], [0usize; 0]);
        assert_eq!(&sample_indices_inplace(&mut r, 1, 0)[..], [0usize; 0]);
        assert_eq!(&sample_indices_inplace(&mut r, 1, 1)[..], [0]);

        assert_eq!(&sample_indices_cache(&mut r, 0, 0)[..], [0usize; 0]);
        assert_eq!(&sample_indices_cache(&mut r, 1, 0)[..], [0usize; 0]);
        assert_eq!(&sample_indices_cache(&mut r, 1, 1)[..], [0]);

        // Make sure lucky 777's aren't lucky
        let slice = &[42, 777];
        let mut num_42 = 0;
        let total = 1000;
        for _ in 0..total {
            let v = sample_slice(&mut r, slice, 1);
            assert_eq!(v.len(), 1);
            let v = v[0];
            assert!(v == 42 || v == 777);
            if v == 42 {
                num_42 += 1;
            }
        }
        let ratio_42 = num_42 as f64 / 1000 as f64;
        assert!(0.4 <= ratio_42 || ratio_42 <= 0.6, "{}", ratio_42);
    }

    #[test]
    #[cfg(feature = "alloc")]
    #[allow(deprecated)]
    fn test_sample_slice() {
        let xor_rng = XorShiftRng::from_seed;

        let max_range = 100;
        let mut r = ::test::rng(403);

        for length in 1usize..max_range {
            let amount = r.gen_range(0, length);
            let mut seed = [0u8; 16];
            r.fill(&mut seed);

            // assert that the two index methods give exactly the same result
            let inplace = sample_indices_inplace(
                &mut xor_rng(seed), length, amount);
            let cache = sample_indices_cache(
                &mut xor_rng(seed), length, amount);
            assert_eq!(inplace, cache);

            // assert the basics work
            let regular = sample_indices(
                &mut xor_rng(seed), length, amount);
            assert_eq!(regular.len(), amount);
            assert!(regular.iter().all(|e| *e < length));
            assert_eq!(regular, inplace);

            // also test that sampling the slice works
            let vec: Vec<usize> = (0..length).collect();
            {
                let result = sample_slice(&mut xor_rng(seed), &vec, amount);
                assert_eq!(result, regular);
            }

            {
                let result = sample_slice_ref(&mut xor_rng(seed), &vec, amount);
                let expected = regular.iter().map(|v| v).collect::<Vec<_>>();
                assert_eq!(result, expected);
            }
        }
    }
}
