// Sample data for the Welcome screen, copied verbatim from the mock's
// `<script type="text/x-dc">` `defs` array (design/screens/Welcome.dc.html,
// docs/01-MOCK-SPEC.md §8). The visible strings are the SAME words as the mock;
// the component renders them through i18n (welcome.opt.*), so these titleKey/subKey
// point at those keys — the raw English kept in comments matches the mock 1:1.

/** One "how to begin" radio option. */
export interface WelcomeOption {
  /** mode value: 'setup' | 'join' | 'recover'. */
  id: 'setup' | 'join' | 'recover'
  /** i18n key for the option title. */
  titleKey: string
  /** i18n key for the option sub-line. */
  subKey: string
}

/** All the data the Welcome screen needs from a fixture / seed. */
export interface WelcomeData {
  options: WelcomeOption[]
}

// defs (verbatim from the mock):
//   ['setup',   'Set up my school',            'For the Principal · uses the activation code from your purchase'],
//   ['join',    'Join my school',              'For teachers and accountants · uses your invitation'],
//   ['recover', 'Recover an existing school',  'Move Vidya to a new computer from a backup']
export const welcomeFixture: WelcomeData = {
  options: [
    { id: 'setup', titleKey: 'welcome.opt.setup.title', subKey: 'welcome.opt.setup.sub' },
    { id: 'join', titleKey: 'welcome.opt.join.title', subKey: 'welcome.opt.join.sub' },
    { id: 'recover', titleKey: 'welcome.opt.recover.title', subKey: 'welcome.opt.recover.sub' },
  ],
}
