<script lang="ts" setup>
import type { VxeGridProps } from '#/adapter/vxe-table';
import { useRouter } from 'vue-router';

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

const gridOptions: VxeGridProps<RowType> = {
    checkboxConfig: {
        highlight: true,
        labelField: 'name',
    },
    columns: [
        { title: 'No', type: 'seq', width: 50 },
        { align: 'left', title: 'Name', type: 'checkbox', width: 160 },
        { field: 'description', sortable: true, title: 'Description' },
        { field: 'gmt_create', sortable: true, title: 'Create Time' },
        { field: 'gmt_modified', sortable: true, title: 'Modified Time' },
        { slots: { default: 'action' }, title: 'Actions', width: 160 },
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
        content: 'Are you sure to delete this item?',
        icon: 'question',
    })
        .then(() => {

            message.success({
                content: `Delete cluster successfully!`,
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
        <Grid :table-title="$t('page.cluster.list')" table-title-help="提示">
            <template #toolbar-tools>
                <Button class="mr-2" type="primary" @click="onAdd()">
                    Add
                </Button>
                <Button class="mr-2" type="primary" @click="() => gridApi.query()">
                    刷新当前页面
                </Button>
                <Button type="primary" @click="() => gridApi.reload()">
                    刷新并返回第一页
                </Button>
            </template>
            <template #action="{ row }">
                <Button type="link" @click="onEdit(row)">Edit</Button>
                <Button type="link" @click="onDelete(row)">Delete</Button>
            </template>
        </Grid>
    </Page>
</template>
