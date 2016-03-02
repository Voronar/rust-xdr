extern crate xdr_codec;
extern crate quickcheck;

use std::io::Cursor;
use std::fmt::Debug;
use std::iter;

use xdr_codec::{Pack, Unpack, Error, padding, pack_array, unpack_array, pack_opaque_array, unpack_opaque_array};
use quickcheck::{quickcheck, Arbitrary};

// Output of packing is a multiple of 4
fn pack<T>(v: T) -> bool
    where T: PartialEq + Pack<Cursor<Vec<u8>>>
{
    let mut data = Cursor::new(Vec::new());

    let sz = v.pack(&mut data).expect("pack failed");
    sz % 4 == 0
}

// Packing something then unpacking returns the same value
fn codec<T>(v: T) -> bool
    where T: PartialEq + Pack<Cursor<Vec<u8>>> + Unpack<Cursor<Vec<u8>>>
 {
    let mut data = Cursor::new(Vec::new());

    let psz = v.pack(&mut data).expect("pack failed");

    let mut data = Cursor::new(data.into_inner());
    let (uv, usz) = T::unpack(&mut data).expect("unpack failed");

    psz == usz && v == uv
}

// Packing something then unpacking returns the same value
fn short_unpack<T>(v: T) -> bool
    where T: PartialEq + Pack<Cursor<Vec<u8>>> + Unpack<Cursor<Vec<u8>>>
 {
    let mut data = Cursor::new(Vec::new());

    let psz = v.pack(&mut data).expect("pack failed");

    // truncate data to make sure unpacking fails
    let data = data.into_inner();
    assert_eq!(psz, data.len());
    let data = Vec::from(&data[..data.len()-1]);

    let mut data = Cursor::new(data);
    match T::unpack(&mut data) {
        Err(Error::Byteorder(_)) => true,
        _ => false,
    }
}

fn quickcheck_pack_t<T>()
    where T: PartialEq + Pack<Cursor<Vec<u8>>> + Unpack<Cursor<Vec<u8>>> + Arbitrary + Debug
{
    quickcheck(pack as fn(T) -> bool);
    quickcheck(pack as fn(Vec<T>) -> bool);
    quickcheck(pack as fn(Option<T>) -> bool);
    quickcheck(pack as fn(Vec<Option<T>>) -> bool);
    quickcheck(pack as fn(Option<Vec<T>>) -> bool);
}

fn quickcheck_codec_t<T>()
    where T: PartialEq + Pack<Cursor<Vec<u8>>> + Unpack<Cursor<Vec<u8>>> + Arbitrary + Debug
{
    quickcheck(codec as fn(T) -> bool);
    quickcheck(codec as fn(Vec<T>) -> bool);
    quickcheck(codec as fn(Option<T>) -> bool);
    quickcheck(codec as fn(Vec<Option<T>>) -> bool);
    quickcheck(codec as fn(Option<Vec<T>>) -> bool);
}

fn quickcheck_short_unpack_t<T>()
    where T: PartialEq + Pack<Cursor<Vec<u8>>> + Unpack<Cursor<Vec<u8>>> + Arbitrary + Debug
{
    quickcheck(short_unpack as fn(T) -> bool);
    quickcheck(short_unpack as fn(Vec<T>) -> bool);
    quickcheck(short_unpack as fn(Option<T>) -> bool);
    quickcheck(short_unpack as fn(Vec<Option<T>>) -> bool);
    quickcheck(short_unpack as fn(Option<Vec<T>>) -> bool);
}

#[test]
fn quickcheck_pack_ui32() {
    quickcheck_pack_t::<i32>();
    quickcheck_pack_t::<u32>();
    quickcheck_pack_t::<usize>();
}

#[test]
fn quickcheck_pack_iu64() {
    quickcheck_pack_t::<i64>();
    quickcheck_pack_t::<u64>();
}

#[test]
fn quickcheck_pack_float() {
    quickcheck_pack_t::<f32>();
    quickcheck_pack_t::<f64>();
}

#[test]
fn quickcheck_codec_ui32() {
    quickcheck_codec_t::<i32>();
    quickcheck_codec_t::<u32>();
    quickcheck_codec_t::<usize>();
}

#[test]
fn quickcheck_codec_iu64() {
    quickcheck_codec_t::<i64>();
    quickcheck_codec_t::<u64>();
}

#[test]
fn quickcheck_codec_float() {
    quickcheck_codec_t::<f32>();
    quickcheck_codec_t::<f64>();
}

#[test]
fn quickcheck_short_unpack_ui32() {
    quickcheck_short_unpack_t::<i32>();
    quickcheck_short_unpack_t::<u32>();
    quickcheck_short_unpack_t::<usize>();
}

#[test]
fn quickcheck_short_unpack_iu64() {
    quickcheck_short_unpack_t::<i64>();
    quickcheck_short_unpack_t::<u64>();
}

#[test]
fn quickcheck_short_unpack_float() {
    quickcheck_short_unpack_t::<f32>();
    quickcheck_short_unpack_t::<f64>();
}

fn check_array(arraysz: usize, rxsize: usize, data: Vec<u32>) -> bool {
    let mut buf = Vec::new();

    // pack data we have into the array
    let tsz = pack_array(&data[..], arraysz, &mut buf).expect("pack_array failed");
    if tsz != arraysz * 4 { println!("tsz {} arraysz*4 {}", tsz, arraysz*4); return false }
    if buf.len() != tsz { println!("buf.len {} tsz {}", buf.len(), tsz); return false }

    // if data is shorter than array, then serialized is padded with zero
    if data.len() < arraysz {
        if buf[data.len()*4..].iter().any(|b| *b != 0) { println!("nonzero pad"); return false }
    }

    let mut recv: Vec<u32> = iter::repeat(0xffff_ffff_u32).take(rxsize).collect();
    let mut cur = Cursor::new(buf);

    // unpack rxsize elements
    let rsz = unpack_array(&mut cur, &mut recv[..], arraysz).expect("unpack_array failed");
    if rsz != arraysz * 4 { println!("rsz {} arraysz*4 {}", rsz, arraysz*4); return false }

    // data and recv must match their common prefix up to arraysz
    if data.iter().zip(recv.iter().take(arraysz)).any(|(d, r)| *d != *r) { println!("nonmatching\ndata {:?}\nrecv {:?}", data, recv); return false }

    // if recv is larger than array, then tail is defaulted
    if rxsize > arraysz {
        if recv[arraysz..].iter().any(|v| *v != u32::default()) { println!("nondefault tail"); return false }
    }

    true
}

#[test]
fn quickcheck_array() {
    quickcheck(check_array as fn(usize, usize, Vec<u32>) -> bool);
}

fn check_opaque(arraysz: usize, rxsize: usize, data: Vec<u8>) -> bool {
    let mut buf = Vec::new();

    // pack data we have into the array
    let tsz = pack_opaque_array(&data[..], arraysz, &mut buf).expect("pack_array failed");
    if tsz != arraysz + padding(arraysz).len() { println!("tsz {} arraysz+pad {}", tsz, arraysz+padding(arraysz).len()); return false }
    if buf.len() != tsz { println!("buf.len {} tsz {}", buf.len(), tsz); return false }

    // if data is shorter than array, then serialized is padded with zero
    if data.len() < arraysz {
        if buf[data.len()..].iter().any(|b| *b != 0) { println!("nonzero pad"); return false }
    }

    let mut recv: Vec<u8> = iter::repeat(0xff).take(rxsize).collect();
    let mut cur = Cursor::new(buf);

    // unpack rxsize elements
    let rsz = unpack_opaque_array(&mut cur, &mut recv[..], arraysz).expect("unpack_array failed");
    if rsz != arraysz+padding(arraysz).len() { println!("rsz {} arraysz+pad {}", rsz, arraysz+padding(arraysz).len()); return false }

    // data and recv must match their common prefix up to arraysz
    if data.iter().zip(recv.iter().take(arraysz)).any(|(d, r)| *d != *r) { println!("nonmatching\ndata {:?}\nrecv {:?}", data, recv); return false }

    // if recv is larger than array, then tail is zero
    if rxsize > arraysz {
        if recv[arraysz..].iter().any(|v| *v != 0) { println!("nondefault tail"); return false }
    }

    true
}

#[test]
fn quickcheck_opaque() {
    quickcheck(check_opaque as fn(usize, usize, Vec<u8>) -> bool);
}
