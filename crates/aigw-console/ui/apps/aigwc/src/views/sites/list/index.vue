<script lang="ts" setup>
import type { VxeGridProps } from '#/adapter/vxe-table';
import { useRouter } from 'vue-router';
import { ref } from 'vue';
import { $t } from '#/locales';

import { confirm, Page } from '@vben/common-ui';

import { message, Button } from 'ant-design-vue';

import { useVbenVxeGrid } from '#/adapter/vxe-table';
import { getSiteTableApi, deleteSiteApi, getAllClustersApi } from '#/api';
import { useVbenForm } from '#/adapter/form';


const isClusterLoaded = ref(false);
const clusterRef = ref<string | null>(null);

const [ClusterForm, _formApi] = useVbenForm({
  schema: [
    {
      component: 'ApiSelect',
      componentProps: {
        afterFetch: (data: { name: string; }[]) => {
          const options = data.map(item => ({ label: item.name, value: item.name }));
          if (options.length > 0 && !clusterRef.value) {
            clusterRef.value = options[0]!.value;
          }
          isClusterLoaded.value = true;
          return options;
        },
        api: getAllClustersApi,
        onChange: (value: string, _prevValue: string) => {
          clusterRef.value = value;
          gridApi.reload();
        },
        autoSelect: 'first',
      },
      fieldName: 'cluster',
      label: '',

    },
  ],
  showDefaultActions: false,
});

interface RowType {
  name: string;
  root_dir: string;
  alt_names: string;
  tls_on: boolean;
  tls_cert_start_date: string;
  tls_cert_end_date: string;
}

const gridOptions: VxeGridProps<RowType> = {
  columns: [
    { title: 'No', type: 'seq', width: 50 },
    { field: 'name', title: $t('page.site.name'),align:"left", width: 160 },
    { field: 'alt_names', align:"left", title: $t('page.site.alternativeNames') },
    { field: 'root_dir', align:"left", title: $t('page.site.rootDir') },
    { field: 'tls_on', cellRender: { name: 'CellTag' }, title: $t('page.site.tlsOn') },
    { slots: { default: 'tls_cert_date' }, title: $t('page.site.tlsCertDate') },
    { slots: { default: 'action' }, title: $t('page.operation'), width: 160 },
  ],
  exportConfig: {},
  keepSource: true,
  proxyConfig: {
    ajax: {
      query: async ({ page, sort }) => {
        var cluster = clusterRef.value;
        let data = await getSiteTableApi(cluster, {
          page: page.currentPage,
          page_size: page.pageSize,
          sort_by: sort.field,
          sort_order: sort.order,
        });

        return data;
      },
    },
    sort: true,
  },
  sortConfig: {
    defaultSort: { field: 'category', order: 'desc' },
    remote: true,
  },
  toolbarConfig: {
    custom: true,
    export: true,
    refresh: true,
    refreshOptions: { code: 'query' },
    zoom: true,
  },
};

const [Grid, gridApi] = useVbenVxeGrid({
  gridOptions,
});


const router = useRouter();

const onAdd = () => {
  router.push('/sites/add');
};

const onEdit = (row: RowType) => {
  router.push('/sites/edit/' + row.name);
};

const onDelete = async (row: RowType) => {

  confirm({
    beforeClose({ isConfirm }) {
      if (!isConfirm) return;
      return deleteSiteApi(row.name);
    },
    centered: false,
    content: $t('page.deleteConfirm'),
    icon: 'question',
  })
    .then(() => {

      message.success({
        content: $t('page.site.deleteSuccess'),
      });
      gridApi.reload()
    })
    .catch(() => {
      // cancel
    });



};

</script>

<template>
  <Page auto-content-height content-class="flex flex-col gap-4" :title="$t('page.site.list')">
    <template #description>
      <div class="text-muted-foreground">
        <p>
          {{ $t('page.site.tip') }}
        </p>
      </div>
    </template>
    <template #extra>
      <ClusterForm class="mb-2" />
    </template>

    <div v-if="isClusterLoaded">
      <Grid :table-title="$t('page.site.list')">
        <template #toolbar-tools>
          <Button class="mr-2" type="primary" @click="onAdd()">
            {{ $t('page.new') }}
          </Button>
          <Button class="mr-2" type="primary" @click="() => gridApi.query()">
            {{ $t('page.refreshCurrentPage') }}
          </Button>
          <Button type="primary" @click="() => gridApi.reload()">
            {{ $t('page.refreshAndReturnFirst') }}
          </Button>
        </template>
        <template #tls_cert_date="{ row }">
          {{ row.tls_cert_start_date }} - {{ row.tls_cert_end_date }}
        </template>
        <template #action="{ row }">
          <Button type="link" @click="onEdit(row)">Edit</Button>
          <Button type="link" @click="onDelete(row)">Delete</Button>
        </template>
      </Grid>
    </div>

  </Page>
</template>
