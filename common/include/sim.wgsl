@group(0) @binding(0) var<storage, read> nodes_old_: array<u32>;
@group(0) @binding(1) var<storage, write> nodes_new_: array<u32>;

@compute @workgroup_size(8, 8, 1)
fn update(@builtin(global_invocation_id) inv_id: vec3<u32>) {
	var lower: u32 = nodes_old_[inv_id * 2];
	var upper: u32 = nodes_old_[inv_id * 2 + 1];
	
	let old_state: u32/* u8 */ = (lower & 0xFF000000) >> 24;
	let new_state: u32/* u8 */ = old_state;
	
	let new_lower: u32 = lower & 0x00FFFFFF | (new_state << 24);
	nodes_new_[inv_id * 2] = new_lower;
	nodes_new_[inv_id * 2 + 1] = upper;
}
