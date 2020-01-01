#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct Permission {
    perm: u16,
}

impl Permission {
    pub fn set(&mut self, accessor: Accessor, access: Access, enabled: bool) {
        use Accessor::*;
        use Access::*;
        let accessors = [User, Group, Other];
        let access_types = [Read, Write, Execute];
        let bit = 1 << (3 * accessors.iter().position(|a| a == &accessor).unwrap()
                        + access_types.iter().position(|a| a == &access).unwrap());
        if enabled {
            self.perm |= bit;
        } else {
            self.perm &= !bit;
        }
    }
}

impl Default for Permission {
    fn default() -> Self {
        Self {
            perm: 0,
        }
    }
}

#[derive(PartialEq)]
pub enum Accessor {
    User,
    Group,
    Other,
}

#[derive(PartialEq)]
pub enum Access {
    Read,
    Write,
    Execute,
}

