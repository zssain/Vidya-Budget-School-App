import type { Bundle } from '../index'

// Phase 4 screens: Staff & access, Conflict review, Sync & devices.
const p04: Bundle = {
  en: {
    // Staff & access.
    'sa.title': 'Staff & access',
    'sa.add': 'Add staff',
    'sa.name': 'Name',
    'sa.role': 'Role',
    'sa.mobile': 'Mobile',
    'sa.email': 'Google email (optional)',
    'sa.suspend': 'Suspend',
    'sa.remove': 'Remove',
    'sa.resend': 'Resend invite',
    'sa.revokeInvite': 'Revoke invite',
    'sa.invite': 'Invite',
    'sa.assignments': 'Assignments',
    'sa.access': 'Effective access',
    'sa.inviteCode': 'Invite code',
    'sa.inviteLink': 'Invite link',
    'sa.fingerprint': 'Fingerprint',
    'sa.expires': 'Expires',
    'sa.state.invited': 'Invited',
    'sa.state.active': 'Active',
    'sa.state.suspended': 'Suspended',
    'sa.cancel': 'Cancel',
    'sa.save': 'Add & create invite',

    // Conflict review.
    'cr.title': 'Conflict review',
    'cr.keepA': 'Keep current',
    'cr.keepB': 'Keep incoming',
    'cr.edit': 'Edit',
    'cr.field': 'Field',
    'cr.current': 'Current value',
    'cr.incoming': 'Incoming value',
    'cr.empty': 'No conflicts to review.',
    'cr.flags': 'Needs a check',
    'cr.markChecked': 'Mark as checked',
    'cr.flagsEmpty': 'Nothing to check.',
    'cr.resolve': 'Resolve',

    // Sync & devices.
    'sd.title': 'Sync & devices',
    'sd.server': 'This computer (school server)',
    'sd.status': 'Status',
    'sd.addresses': 'Addresses',
    'sd.port': 'Port',
    'sd.fingerprint': 'Fingerprint',
    'sd.devices': 'Devices',
    'sd.revoke': 'Revoke',
    'sd.owner': 'Owner',
    'sd.platform': 'Platform',
    'sd.series': 'Series',
    'sd.lastSeen': 'last seen',
    'sd.pending': 'Waiting to send',
    'sd.waiting': 'waiting for server',
    'sd.syncNow': 'Sync now',
    'sd.online': 'Online',
    'sd.revoked': 'Revoked',
  },
  hi: {},
}

p04.hi = Object.fromEntries(Object.entries(p04.en).map(([k, v]) => [k, `TODO-HI: ${v}`]))

export default p04
