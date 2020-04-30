use std::ptr::NonNull;

#[repr(C)]
pub struct PortOp {
	port: u32,
	buf_len: u32,
	buf: Option<NonNull<u8>>,
}


#[no_mangle]
pub extern "C" fn do_sync(
	_ops_ptr: Option<NonNull<PortOp>>,
	_ops_len: usize,
	_batch_ptr: Option<NonNull<u32>>,
	_batch_len: usize)
{
	// TODO
}