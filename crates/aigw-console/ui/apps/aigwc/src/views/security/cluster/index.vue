<script lang="ts" setup>
import type { VxeTableGridOptions } from '#/adapter/vxe-table';
import { useRouter } from 'vue-router';
import { $t } from '#/locales';
import { confirm, Page } from '@vben/common-ui';
import { watch, ref } from 'vue';
import { message, Button, Tabs, TabPane } from 'ant-design-vue';

import { useVbenVxeGrid } from '#/adapter/vxe-table';
import { getClusterIpTableApi, deleteClusterIpApi } from '#/api';
import { clusterStore } from '#/store';

import {
    createIconifyIcon,
} from '@vben/icons';
const Shield = createIconifyIcon('lucide:shield');
const ShieldBan = createIconifyIcon('lucide:shield-ban');

let clusterAccess = clusterStore();

const activeKey = ref('1');

interface RowType {
    id: number;
    cluster_name: string;
    ip: string;
    prefix_len: number;
    start_time: string,
    end_time: string,
    gmt_modified: string;
}

const gridOptions: VxeTableGridOptions<RowType> = {
    columns: [
        { title: 'No', type: 'seq', width: 50 },
        { slots: { default: 'ip' }, title: "IP", align: "left", width: 320 },
        { slots: { default: 'cluster_name' }, align: "left", title: $t('page.cluster.name') },
        { field: 'start_time', title: $t('page.security.startTime') },
        { field: 'end_time', title: $t('page.security.endTime') },
        { field: 'gmt_modified', sortable: true, title: $t('page.modifiedTime') },
        { slots: { default: 'action' }, title: $t('page.operation'), width: 160 },
    ],
    exportConfig: {},
    height: '',
    keepSource: true,
    proxyConfig: {
        ajax: {
            query: async ({ page, sort }) => {
                var cluster = clusterAccess.current!;
                let data = await getClusterIpTableApi(cluster, activeKey.value!, {
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
    router.push('/security/cluster/ip/add?type=' + activeKey.value);
};

const onClusterEdit = (row: RowType) => {
    router.push('/sites/clusters/edit/' + row.cluster_name);
};

const onDelete = async (row: RowType) => {

    confirm({
        beforeClose({ isConfirm }) {
            if (!isConfirm) return;
            return deleteClusterIpApi(row.id);
        },
        centered: false,
        content: $t('page.deleteConfirm'),
        icon: 'question',
    })
        .then(() => {
            message.success({
                content: $t('page.security.cluster.deleteSuccess'),
            });
            gridApi.reload()
        })
        .catch(() => {
            // cancel
        });

};

watch(
    () => clusterAccess.current,
    (newCluster, oldCluster) => {
        if (newCluster !== oldCluster) {
            if (oldCluster !== undefined || newCluster !== null) {
                gridApi?.reload();
            }
        }
    }
);
watch(activeKey, (newType, oldType) => {
    if (newType !== oldType) {
        gridApi?.reload();
    }
});

</script>

<template>
    <Page auto-content-height>

        <Tabs v-model:activeKey="activeKey">
            <TabPane key="1" type="card">
                <template #tab>
                    <span class="flex items-center">
                        <Shield class="mr-1" style="width: 16px; height: 16px;" />
                        {{ $t('page.security.cluster.ipWhiteList') }}
                    </span>

                </template>

                <Grid :table-title="$t('page.security.cluster.ipWhiteList')">
                    <template #toolbar-tools>
                        <Button class="mr-2" type="primary" @click="onAdd()">
                            {{ $t('page.add') }}
                        </Button>
                    </template>
                    <template #cluster_name="{ row }">
                        <Button type="link" @click="onClusterEdit(row)">{{ row.cluster_name }}</Button>
                    </template>
                    <template #action="{ row }">
                        <Button type="link" @click="onDelete(row)">{{ $t('page.delete') }}</Button>
                    </template>
                    <template #ip="{ row }">
                        {{ row.ip }} / {{ row.prefix_len }}
                    </template>
                </Grid>
            </TabPane>
            <TabPane key="2" type="card">
                <template #tab>
                    <span class="flex items-center">
                        <ShieldBan class="mr-1" style="width: 16px; height: 16px;" />
                        {{ $t('page.security.cluster.ipBlockList') }}
                    </span>

                </template>
                <Grid :table-title="$t('page.security.cluster.ipBlockList')">
                    <template #toolbar-tools>
                        <Button class="mr-2" type="primary" @click="onAdd()">
                            {{ $t('page.add') }}
                        </Button>
                    </template>
                    <template #cluster_name="{ row }">
                        <Button type="link" @click="onClusterEdit(row)">{{ row.cluster_name }}</Button>
                    </template>
                    <template #action="{ row }">
                        <Button type="link" @click="onDelete(row)">{{ $t('page.delete') }}</Button>
                    </template>
                    <template #ip="{ row }">
                        {{ row.ip }} / {{ row.prefix_len }}
                    </template>
                </Grid>
            </TabPane>
        </Tabs>

    </Page>

</template>
