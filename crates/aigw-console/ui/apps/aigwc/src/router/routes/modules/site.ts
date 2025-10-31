import type { RouteRecordRaw } from 'vue-router';

import { $t } from '#/locales';

const routes: RouteRecordRaw[] = [
  {
    meta: {
      icon: 'lucide:globe',
      order: 3,
      title: $t('page.site.title'),
    },
    name: 'Site',
    path: '/site',
    children: [
      {
        name: 'cluster',
        path: '/sites/clusters',
        component: () => import('#/views/clusters/list/index.vue'),
        meta: {
          icon: 'lucide:server',
          title: $t('page.cluster.list'),
        },
      },
      {
        name: 'Cluster Edit',
        path: '/sites/clusters/edit/:id',
        component: () =>
          import('#/views/clusters/edit/index.vue'),
        meta: {
          activePath: '/sites/clusters',
          hideInMenu: true,
          maxNumOfOpenTab: 1,
          title: $t('page.cluster.edit'),
        },
      },
      {
        name: 'Cluster Add',
        path: '/sites/clusters/add',
        component: () => import('#/views/clusters/new/index.vue'),
        meta: {
          activePath: '/sites/clusters',
          hideInMenu: true,
          maxNumOfOpenTab: 1,
          title: $t('page.cluster.new'),
        },
      },
      {
        name: 'sites',
        path: '/sites',
        component: () => import('#/views/sites/list/index.vue'),
        meta: {
          affixTab: true,
          icon: 'lucide:list',
          title: $t('page.site.list'),
        },
      },
      {
        name: 'Site Edit',
        path: '/sites/edit/:id',
        component: () =>
          import('#/views/sites/edit/index.vue'),
        meta: {
          activePath: '/sites',
          hideInMenu: true,
          maxNumOfOpenTab: 1,
          title: $t('page.site.edit'),
        },
      },
      {
        name: 'Site Add',
        path: '/sites/add',
        component: () => import('#/views/sites/new/index.vue'),
        meta: {
          icon: 'lucide:plus',
          title: $t('page.site.new'),
        },
      },
    ],
  },
];

export default routes;
