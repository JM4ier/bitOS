/// UNIX style permission:
/// It stors the access rights of the user owning the file,
/// the groups that the user belongs to as well as others
/// in regard to reading, writing and executing the file.
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct Permission {
    /// bitmap of access rights
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
        let mut perm = Self {
            perm: 0,
        };
        perm.set(Accessor::User, Access::Read, true);
        perm.set(Accessor::User, Access::Write, true);
        perm
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

