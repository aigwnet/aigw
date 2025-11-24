<script lang="ts" setup>
import type { VxeTableGridOptions } from '#/adapter/vxe-table';
import { useRouter } from 'vue-router';
import { $t } from '#/locales';
import { confirm, Page } from '@vben/common-ui';

import { message, Button } from 'ant-design-vue';
import { createIconifyIcon } from '@vben/icons';
import { useVbenVxeGrid } from '#/adapter/vxe-table';
import { getClusterTableApi, deleteClusterApi } from '#/api';

const CopyIcon = createIconifyIcon('ant-design:copy-outlined');
const EditIcon = createIconifyIcon('ant-design:edit-outlined');
const DeleteIcon = createIconifyIcon('ant-design:delete-outlined');


interface RowType {
    id: number;
    name: string;
    security_key: string;
    description: string;
    gmt_create: string;
    gmt_modified: string;
}

const gridOptions: VxeTableGridOptions<RowType> = {
    columns: [
        { title: 'No', type: 'seq', width: 50 },
        { field: 'name', title: $t('page.cluster.name'), align: "left", width: 160 },
        { slots: { default: 'security_key' }, title: $t('page.cluster.key'), align: "left" },
        { field: 'enable', cellRender: { name: 'CellTag' }, title: $t('page.cluster.enable') },
        { field: 'enable_default_site', cellRender: { name: 'CellTag' }, title: $t('page.cluster.enableDefaultSite') },
        { field: 'description', align: "left", title: $t('page.cluster.description') },
        { field: 'gmt_modified', sortable: true, title: $t('page.modifiedTime') },
        { slots: { default: 'action' }, title: $t('page.operation'), width: 160 },
    ],
    exportConfig: {},
    height: 'auto',
    keepSource: true,
    proxyConfig: {
        ajax: {
            query: async ({ page, sort }) => {
                let data = await getClusterTableApi({
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
    router.push('/sites/clusters/add');
};

const onEdit = (row: RowType) => {
    router.push('/sites/clusters/edit/' + row.name);
};

const onDelete = async (row: RowType) => {

    confirm({
        beforeClose({ isConfirm }) {
            if (!isConfirm) return;
            return deleteClusterApi(row.name);
        },
        centered: false,
        content: $t('page.deleteConfirm'),
        icon: 'question',
    })
        .then(() => {
            message.success({
                content: $t('page.cluster.deleteSuccess'),
            });
            gridApi.reload()
        })
        .catch(() => {
            // cancel
        });
};

const copySecurityKey = async (key: string) => {
    if (!key) {
        return;
    }
    try {
        await navigator.clipboard.writeText(key);
        message.success($t('page.copySuccess'));
    } catch (err) {
        console.error('Failed to copy:', err);
    }
};

</script>

<template>
    <Page auto-content-height content-class="flex flex-col gap-4" :title="$t('page.cluster.list')">
        <template #description>
            <div class="text-muted-foreground">
                <p>
                    {{ $t('page.cluster.tip') }}
                </p>
            </div>
        </template>
        <Grid :table-title="$t('page.cluster.list')">
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
            <template #security_key="{ row }">
                <span>••••••••••••</span> &nbsp;
                <Button shape="circle" size="small" :title="$t('page.copy')"
                    @click.stop="copySecurityKey(row.security_key)">
                    <CopyIcon />
                </Button>
            </template>
            <template #action="{ row }">
                <Button shape="circle" size="small" @click="onEdit(row)" :title="$t('page.edit')">
                    <EditIcon />
                </Button> &nbsp;
                <Button shape="circle" size="small" danger @click="onDelete(row)" :title="$t('page.delete')">
                    <DeleteIcon />
                </Button>
            </template>
        </Grid>
    </Page>
</template>
