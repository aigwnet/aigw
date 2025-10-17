<script lang="ts" setup>
import { ref } from 'vue';
import {
  Page, AnalysisChartCard
} from '@vben/common-ui';
import { Button } from 'ant-design-vue';
import { getAllClustersApi, getServerTableApi, getAnalyticsMonitorApi, type AnalyticsApi } from '#/api';
import { useVbenForm } from '#/adapter/form';
import type { VxeGridProps } from '#/adapter/vxe-table';
import { useVbenVxeGrid } from '#/adapter/vxe-table';
import { useRouter } from 'vue-router';

import MonitorCpu from './monitor-cpu.vue';
import MonitorCpuLoad from './monitor-cpu-load.vue';
import MonitorMem from './monitor-mem.vue';
import MonitorDisk from './monitor-disk.vue';
import MonitorIO from './monitor-io.vue';
import MonitorNet from './monitor-network.vue';
import MonitorRt from './monitor-rt.vue';
import MonitorError from './monitor-error.vue';

const isClusterLoaded = ref(false);
const clusterRef = ref<string | null>(null);
const analyticsMonitor = ref<Array<AnalyticsApi.AnalyticsMonitor>>([]);

async function loadAnalyticsMonitor(cluster: string) {
  const items = await getAnalyticsMonitorApi(cluster);
  analyticsMonitor.value = items;
};

const [ClusterForm, _formApi] = useVbenForm({
  schema: [
    {
      component: 'ApiSelect',
      componentProps: {
        afterFetch: (data: { name: string; }[]) => {
          const options = data.map(item => ({ label: item.name, value: item.name }));
          if (options.length > 0 && !clusterRef.value) {
            clusterRef.value = options[0]!.value;
            loadAnalyticsMonitor(clusterRef.value);
          }
          isClusterLoaded.value = true;
          return options;
        },
        api: getAllClustersApi,
        onChange: (value: string, _prevValue: string) => {
          clusterRef.value = value;
          gridApi.reload();
          loadAnalyticsMonitor(value)
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
  ip: string;
  version: string;
  os_name: string;
  os_version: string;
  os_arch: string;
  cpu_name: string;
  cpu_vendor: string;
  cpu_frequency: number;
  cpu_nums: number;
  status: string;
  gmt_modified: string;
}

const gridOptions: VxeGridProps<RowType> = {
  checkboxConfig: {
    highlight: true,
    labelField: 'name',
  },
  columns: [
    { title: 'No', type: 'seq', width: 50 },
    { field: 'ip', sortable: true, title: 'IP' },
    { field: 'version', sortable: true, title: 'Version' },
    { slots: { default: 'os_info' }, title: 'OS' },
    { slots: { default: 'cpu_info' }, title: 'CPU' },
    { field: 'cpu_nums', sortable: true, title: 'CPU核数' },
    { field: 'gmt_modified', sortable: true, title: '修改时间' },
    { slots: { default: 'action' }, title: 'Actions', width: 160 },
  ],
  exportConfig: {},
  //height: 'auto',
  keepSource: true,
  proxyConfig: {
    enabled: true,
    ajax: {
      query: async ({ page, sort }) => {
        var cluster = clusterRef.value;
        let data = await getServerTableApi(cluster, {
          cluster: cluster,
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
    defaultSort: { field: 'ip', order: 'desc' },
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

const onClickServer = (row: RowType) => {
  //router.push('/sites/add');
};
</script>

<template>

  <Page auto-content-height content-class="flex flex-col gap-4" :title="$t('page.dashboard.monitor')">
    <template #description>
      <div class="text-muted-foreground">
        <p>
          查看集群访问情况、集群状态以及服务器详情等信息。
        </p>
      </div>
    </template>
    <template #extra>
      <ClusterForm class="mb-2" />
    </template>

    <div v-if="isClusterLoaded" class="w-full md:flex">
      <AnalysisChartCard class="mt-5 md:mr-4 md:mt-0 md:w-1/2" title="CPU">
        <MonitorCpu :data="analyticsMonitor" />
      </AnalysisChartCard>
      <AnalysisChartCard class="mt-5 md:mt-0 md:w-1/2" title="CPU Load">
        <MonitorCpuLoad :data="analyticsMonitor" />
      </AnalysisChartCard>
    </div>
    <div v-if="isClusterLoaded" class="w-full md:flex">
      <AnalysisChartCard class="mt-5 md:mr-4 md:mt-0 md:w-1/2" title="Memory">
        <MonitorMem :data="analyticsMonitor" />
      </AnalysisChartCard>
      <AnalysisChartCard class="mt-5 md:mt-0 md:w-1/2" title="Disk">
        <MonitorDisk :data="analyticsMonitor" />
      </AnalysisChartCard>
    </div>
    <div class="w-full md:flex">
      <AnalysisChartCard class="mt-5 md:mr-4 md:mt-0 md:w-1/2" title="磁盘IO">
        <MonitorIO :data="analyticsMonitor" />
      </AnalysisChartCard>
      <AnalysisChartCard class="mt-5 md:mt-0 md:w-1/2" title="流量统计">
        <MonitorNet :data="analyticsMonitor" />
      </AnalysisChartCard>
    </div>
    <div class="w-full md:flex">
      <AnalysisChartCard class="mt-5 md:mr-4 md:mt-0 md:w-1/2" title="耗时">
        <MonitorRt :data="analyticsMonitor" />
      </AnalysisChartCard>
      <AnalysisChartCard class="mt-5 md:mt-0 md:w-1/2" title="错误">
        <MonitorError :data="analyticsMonitor" />
      </AnalysisChartCard>
    </div>

    <div v-if="isClusterLoaded">
      <Grid table-title="服务器列表" table-title-help="提示">
        <template #toolbar-tools>
          <Button class="mr-2" type="primary" @click="() => gridApi.query()">
            刷新当前页面
          </Button>
          <Button type="primary" @click="() => gridApi.reload()">
            刷新并返回第一页
          </Button>
        </template>
        <template #os_info="{ row }">
          {{ row.os_name }} - {{ row.os_version }} / {{ row.os_arch }}
        </template>
        <template #cpu_info="{ row }">
          {{ row.cpu_name }}
        </template>
        <template #action="{ row }">
          <Button type="link" @click="onClickServer(row)">查看详情</Button>
        </template>
      </Grid>
    </div>

  </Page>

</template>
