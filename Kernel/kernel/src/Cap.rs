#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct Cap {
    pub obj_type: u32,
    pub obj_id: u32,
    pub rights: u32,
}

pub const CAP_NONE: Cap = Cap {
    obj_type: 0,
    obj_id: 0,
    rights: 0,
};

pub const OBJ_ENDPOINT: u32 = 1;
pub const RIGHT_SEND: u32 = 1;
pub const RIGHT_RECV: u32 = 2;

pub const MAX_CAPS: usize = 32;

pub fn alloc(caps: &mut [Cap; MAX_CAPS], obj_type: u32, obj_id: u32, rights: u32) -> u32 {
    for i in 1..MAX_CAPS {
        if caps[i].obj_type == 0 {
            caps[i] = Cap {
                obj_type,
                obj_id,
                rights,
            };
            return i as u32;
        }
    }
    0
}

pub fn lookup(caps: &[Cap; MAX_CAPS], idx: u32) -> Option<Cap> {
    let i = idx as usize;
    if i == 0 || i >= MAX_CAPS {
        return None;
    }
    let c = caps[i];
    if c.obj_type == 0 {
        None
    } else {
        Some(c)
    }
}

pub fn free(caps: &mut [Cap; MAX_CAPS], idx: u32) {
    let i = idx as usize;
    if i > 0 && i < MAX_CAPS {
        caps[i] = CAP_NONE;
    }
}

pub fn find_by_obj(caps: &[Cap; MAX_CAPS], obj_type: u32, obj_id: u32) -> Option<u32> {
    for i in 1..MAX_CAPS {
        if caps[i].obj_type == obj_type && caps[i].obj_id == obj_id {
            return Some(i as u32);
        }
    }
    None
}

pub fn has_rights(cap: Cap, rights: u32) -> bool {
    cap.rights & rights == rights
}

pub fn count(caps: &[Cap; MAX_CAPS]) -> u32 {
    let mut n: u32 = 0;
    for i in 1..MAX_CAPS {
        if caps[i].obj_type != 0 {
            n += 1;
        }
    }
    n
}
