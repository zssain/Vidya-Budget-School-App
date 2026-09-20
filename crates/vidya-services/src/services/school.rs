//! Read-only school header, available to any signed-in role.

use vidya_core::roles::Actor;
use vidya_db::repo;

use crate::error::ServiceError;
use crate::services::dto::SchoolHeaderDto;
use crate::Services;

pub struct SchoolService<'a> {
    pub services: &'a Services,
}

impl SchoolService<'_> {
    /// The school header (name, address, ids and current session name). Any
    /// signed-in actor may read it; the command layer authenticates the actor.
    pub fn header(&self, _actor: &Actor) -> Result<SchoolHeaderDto, ServiceError> {
        let (school, session) = self
            .services
            .db
            .read(|conn| Ok((repo::school::get(conn)?, repo::sessions::current(conn)?)))?;
        let school = school.ok_or_else(|| ServiceError::internal("school header requested before setup"))?;
        Ok(SchoolHeaderDto {
            name: school.name,
            address: school.address,
            udise: school.udise,
            board: school.board,
            phone: school.phone,
            session_name: session.map(|s| s.name),
        })
    }
}
