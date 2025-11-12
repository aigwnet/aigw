<script lang="ts" setup>
import type { VxeTableGridOptions } from '#/adapter/vxe-table';
import { useRouter } from 'vue-router';
import { $t } from '#/locales';
import { confirm, Page } from '@vben/common-ui';

import { message, Button } from 'ant-design-vue';

import { useVbenVxeGrid } from '#/adapter/vxe-table';
import { getClusterTableApi, deleteClusterApi } from '#/api';

interface RowType {
    id: number;
    name: string;
    description: string;
    gmt_create: string;
    gmt_modified: string;
}

const gridOptions: VxeTableGridOptions<RowType> = {
    columns: [
        { title: 'No', type: 'seq', width: 50 },
        { field: 'name', title: $t('page.cluster.name'), align: "left", width: 160 },
        { field: 'security_key', title: $t('page.cluster.key'), align: "left" },
        { field: 'enable', cellRender: { name: 'CellTag' }, title: $t('page.cluster.enable') },
        { field: 'default_site_enable', cellRender: { name: 'CellTag' }, title: $t('page.cluster.defaultSiteEnable') },
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
    router.push('/sites/clusters/edit/' + row.id);
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

</script>

<template>
    <Page auto-content-height>
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
            <template #action="{ row }">
                <Button type="link" @click="onEdit(row)">{{ $t('page.edit') }}</Button>
                <Button type="link" @click="onDelete(row)">{{ $t('page.delete') }}</Button>
            </template>
        </Grid>
    </Page>
</template>
