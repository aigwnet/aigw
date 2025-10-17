import type { RouteRecordRaw } from 'vue-router';

import { $t } from '#/locales';

const routes: RouteRecordRaw[] = [
  {
    meta: {
      icon: 'lucide:layout-dashboard',
      order: -1,
      title: $t('page.dashboard.title'),
    },
    name: 'Dashboard',
    path: '/dashboard',
    children: [
      {
        name: 'Traffic',
        path: '/dashboard/traffic',
        component: () => import('#/views/dashboard/traffic/index.vue'),
        meta: {
          affixTab: true,
          icon: 'lucide:area-chart',
          title: $t('page.dashboard.traffic'),
        },
      },
      {
        name: 'Monitor',
        path: '/dashboard/mointor',
        component: () => import('#/views/dashboard/monitor/index.vue'),
        meta: {
          affixTab: true,
          icon: 'lucide:view',
          title: $t('page.dashboard.monitor'),
        },
      }
    ],
  },
];

export default routes;
