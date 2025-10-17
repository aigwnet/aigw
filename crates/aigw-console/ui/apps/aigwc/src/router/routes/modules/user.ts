import type { RouteRecordRaw } from 'vue-router';

import { $t } from '#/locales';

const routes: RouteRecordRaw[] = [
  {
    name: 'Profile',
    path: '/user/profile',
    component: () => import('#/views/user/profile.vue'),
    meta: {
      icon: 'lucide:user',
      title: $t('page.user.profile'),
      order: 9999,
    },
  },
];

export default routes;
