<script lang="ts" setup>
import type { VxeTableGridOptions, VxeGridPropTypes } from '#/adapter/vxe-table';
import { useRouter } from 'vue-router';
import { $t } from '#/locales';
import { confirm, Page } from '@vben/common-ui';
import { watch, ref, shallowRef, nextTick, computed } from 'vue';
import { message, Button, Tabs, TabPane } from 'ant-design-vue';

import { useVbenVxeGrid } from '#/adapter/vxe-table';
import { getClusterIpTableApi, deleteClusterIpApi } from '#/api';
import { clusterStore } from '#/store';

import {
    createIconifyIcon,
} from '@vben/icons';
const Shield = createIconifyIcon('lucide:shield');
const ShieldBan = createIconifyIcon('lucide:shield-ban');
const DeleteIcon = createIconifyIcon('ant-design:delete-outlined');

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

const baseColumns: VxeGridPropTypes.Columns<RowType> = [
    { title: 'No', type: 'seq', width: 50 },
    { slots: { default: 'ip' }, type: 'checkbox', title: 'IP', align: 'left', width: 320 },
    { slots: { default: 'cluster_name' }, align: 'left', title: $t('page.cluster.name') },
    { field: 'start_time', title: $t('page.security.startTime') },
    { field: 'end_time', title: $t('page.security.endTime') },
    { field: 'gmt_modified', sortable: true, title: $t('page.modifiedTime') },
    { slots: { default: 'action' }, title: $t('page.operation'), width: 160 },
];

function createGridOptions(type: '1' | '2') {
    return {
        columns: baseColumns,
        exportConfig: {},
        height: '',
        keepSource: true,
        proxyConfig: {
            ajax: {
                query: async ({ page, sort }) => {
                    const cluster = clusterAccess.current;
                    if (!cluster) return { list: [], total: 0 };
                    const data = await getClusterIpTableApi(cluster, type, {
                        page: page.currentPage,
                        page_size: page.pageSize,
                        sort_by: sort?.field,
                        sort_order: sort?.order,
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
    } satisfies VxeTableGridOptions<RowType>;
}

const [GridWhite, gridApiWhite] = useVbenVxeGrid({
    gridOptions: createGridOptions('1'),
});

const [GridBlock, gridApiBlock] = useVbenVxeGrid({
    gridOptions: createGridOptions('2'),
});

const currentGridApi = shallowRef(gridApiWhite);

const safeReload = () => {
    if (currentGridApi.value?.reload && typeof currentGridApi.value.reload === 'function') {
        currentGridApi.value.reload();
    }
};


watch(activeKey, (newKey) => {
    currentGridApi.value = newKey === '1' ? gridApiWhite : gridApiBlock;
    nextTick(() => {
        safeReload();
    });

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
            currentGridApi.value?.reload();
        })
        .catch(() => {
            // cancel
        });

};

const hasSelected = computed(() => currentSelectedRows.value.length > 0);
const selectedRowsWhite = ref<RowType[]>([]);
const selectedRowsBlock = ref<RowType[]>([]);
const currentSelectedRows = computed(() => {
    return activeKey.value === '1' ? selectedRowsWhite.value : selectedRowsBlock.value;
});
const onCheckboxChangeWhite = ({ records }: { records: RowType[] }) => {
    selectedRowsWhite.value = records;
};

const onCheckboxChangeBlock = ({ records }: { records: RowType[] }) => {
    selectedRowsBlock.value = records;
};

const onCheckboxAllWhite = ({ records }: { records: RowType[] }) => {
    selectedRowsWhite.value = records;
};

const onCheckboxAllBlock = ({ records }: { records: RowType[] }) => {
    selectedRowsBlock.value = records;
};

const onBatchDelete = async () => {
    if (currentSelectedRows.value.length === 0) {
        message.warning($t('common.pleaseSelectData'));
        return;
    }

    const ids = currentSelectedRows.value.map(item => item.id);

    confirm({
        beforeClose({ isConfirm }) {
            if (!isConfirm) return;
            //return batchDeleteClusterIpApi(ids);
            return true;
        },
        centered: false,
        content: $t('page.batchDeleteConfirm', { count: currentSelectedRows.value.length }),
        icon: 'question',
    })
        .then(() => {
            message.success({
                content: $t('page.security.cluster.batchDeleteSuccess'),
            });
            // 清空选中状态
            if (activeKey.value === '1') {
                selectedRowsWhite.value = [];
            } else {
                selectedRowsBlock.value = [];
            }
            currentGridApi.value?.reload();
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
                currentGridApi.value?.reload();
            }
        }
    }
);

</script>

<template>
    <Page auto-content-height content-class="flex flex-col gap-4" :title="$t('page.security.cluster.list')">
        <template #description>
            <div class="text-muted-foreground">
                <p>
                    {{ $t('page.security.cluster.tip') }}
                </p>
            </div>
        </template>

        <Tabs v-model:activeKey="activeKey">
            <TabPane key="1" type="card">
                <template #tab>
                    <span class="flex items-center">
                        <Shield class="mr-1" style="width: 16px; height: 16px;" />
                        {{ $t('page.security.cluster.ipWhiteList') }}
                    </span>

                </template>

                <GridWhite :table-title="$t('page.security.cluster.ipWhiteList')" @checkbox-all="onCheckboxAllWhite"
                    @checkbox-change="onCheckboxChangeWhite">
                    <template #toolbar-tools>
                        <Button class="mr-2" type="primary" @click="onAdd()">
                            {{ $t('page.add') }}
                        </Button>
                        <Button class="mr-2" type="primary" danger :disabled="!hasSelected" @click="onBatchDelete">
                            {{ $t('page.deleteSelected') }}
                        </Button>
                    </template>
                    <template #cluster_name="{ row }">
                        <Button type="link" @click="onClusterEdit(row)">{{ row.cluster_name }}</Button>
                    </template>
                    <template #action="{ row }">
                        <Button type="link" @click="onDelete(row)">{{ $t('page.delete') }}</Button>
                    </template>
                    <template #ip="{ row }">
                        {{ row.ip }}/{{ row.prefix_len }}
                    </template>
                </GridWhite>
            </TabPane>
            <TabPane key="2" type="card">
                <template #tab>
                    <span class="flex items-center">
                        <ShieldBan class="mr-1" style="width: 16px; height: 16px;" />
                        {{ $t('page.security.cluster.ipBlockList') }}
                    </span>

                </template>
                <GridBlock :table-title="$t('page.security.cluster.ipBlockList')" @checkbox-all="onCheckboxAllBlock"
                    @checkbox-change="onCheckboxChangeBlock">
                    <template #toolbar-tools>
                        <Button class="mr-2" type="primary" @click="onAdd()">
                            {{ $t('page.add') }}
                        </Button>
                        <Button class="mr-2" type="primary" danger :disabled="!hasSelected" @click="onBatchDelete">
                            {{ $t('page.deleteSelected') }}
                        </Button>
                    </template>
                    <template #cluster_name="{ row }">
                        <Button type="link" @click="onClusterEdit(row)">{{ row.cluster_name }}</Button>
                    </template>
                    <template #action="{ row }">
                        <Button shape="circle" size="small" danger @click="onDelete(row)" :title="$t('page.delete')">
                            <DeleteIcon />
                        </Button>
                    </template>
                    <template #ip="{ row }">
                        {{ row.ip }}/{{ row.prefix_len }}
                    </template>
                </GridBlock>
            </TabPane>
        </Tabs>

    </Page>

</template>