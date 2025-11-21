import type { RouteRecordRaw } from 'vue-router';

import { $t } from '#/locales';

const routes: RouteRecordRaw[] = [
  {
    meta: {
      icon: 'lucide:brick-wall-shield',
      order: 4,
      title: $t('page.security.title'),
    },
    name: 'Security',
    path: '/security',
    children: [
      {
        name: 'clusterIpList',
        path: '/security/cluster',
        component: () => import('#/views/security/cluster/index.vue'),
        meta: {
          icon: 'lucide:server',
          title: $t('page.security.cluster.list'),
        },
      }, 
      {
        name: 'Cluster IP Add',
        path: '/security/cluster/ip/add',
        component: () => import('#/views/security/cluster/new_ip.vue'),
        meta: {
          activePath: '/security/cluster',
          hideInMenu: true,
          maxNumOfOpenTab: 1,
          title: $t('page.security.cluster.newIp'),
        },
      },
    ],
  },
];

export default routes;
