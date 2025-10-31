<script setup lang="ts">
import { ref, watch, h } from 'vue'
import { Card, Row, Col, FormItem, Input, InputNumber, Switch, RadioGroup, Radio, Textarea, Button, Divider } from 'ant-design-vue'
import { Plus, X } from '@vben/icons'
import DynamicHeader from './DynamicHeader.vue'

const props = withDefaults(defineProps<{
    modelValue?: any[]
    min?: number
    max?: number
    namePath: string
}>(), {
    modelValue: () => [],
    min: 1,
    max: 10,
})

const localFields = ref<any[]>([...props.modelValue])

const emit = defineEmits<{
    (e: 'update:modelValue', value: any[]): void
}>()

watch(
    () => props.modelValue,
    (newVal) => {
        if (JSON.stringify(newVal) !== JSON.stringify(localFields.value)) {
            localFields.value = [...(newVal || [])]
        }
    },
    { deep: true }
)

watch(
    localFields,
    (newVal) => {
        emit('update:modelValue', newVal)
    },
    { deep: true }
)

// 添加新项
const addItem = () => {
    if (localFields.value.length >= props.max) return
    localFields.value.push({
        path: '',
        proxy: false,
        protocol: 'http',
        connection_timeout: 5,
        read_timeout: 5,
        write_timeout: 5,
        idle_timeout: 30,
        sni: "",
        client_max_body_size: 0,
        rewrite: "",
        upstream: "",
        root_dir: "",
        auto_index: false,
        proxy_add_headers: [],
        proxy_set_headers: [],
    })
}

// 删除项
const removeItem = (index: number) => {
    if (localFields.value.length <= props.min) return
    localFields.value.splice(index, 1)
}

// 确保最小数量
const ensureMinFields = () => {
    while (localFields.value.length < props.min) {
        localFields.value.push({
            path: '',
            proxy: false,
            protocol: 'http',
            connection_timeout: 5,
            read_timeout: 5,
            write_timeout: 5,
            idle_timeout: 30,
            sni: "",
            client_max_body_size: 0,
            rewrite: "",
            upstream: "",
            root_dir: "",
            auto_index: false,
            proxy_add_headers: [],
            proxy_set_headers: [],
        })
    }
}

// 初始化
ensureMinFields()

const getFieldPath = (index: number, fieldName: string) => {
    return props.namePath + "_" + index + "_" + fieldName
}
</script>

<template>

    <div class="w-full">
        <Button type="primary" v-if="localFields.length < max" dashed @click="addItem" class="mb-4 ">
            <Plus /> Add
        </Button>
        <Card v-for="(_item, index) in localFields" :key="index" class="mb-4 p-4">
            <Row>
                <Col :span="24" class="flex justify-between items-center mb-4">
                <h4 class="text-base font-medium text-blue-600">Location {{ index + 1 }}</h4>
                <Button v-if="localFields.length > min" danger shape="circle" :icon="h(X)" size="small"
                    @click="removeItem(index)" />
                </Col>
            </Row>
            <Row>
                <Col :span="24">
                <FormItem label="Path" :label-col="{ span: 3 }" :name="getFieldPath(index, 'path')">
                    <Input v-model:value="localFields[index].path" placeholder="/" />
                </FormItem>
                </Col>
            </Row>
            <Row>
                <Col :span="24">
                <FormItem label="Proxy" :label-col="{ span: 3 }" :name="getFieldPath(index, 'proxy')">
                    <Switch v-model:checked="localFields[index].proxy" />
                </FormItem>
                </Col>
            </Row>

            <Row v-show="localFields[index].proxy">
                <Col :span="24">
                <FormItem label="Protocol" :label-col="{ span: 3 }" :name="getFieldPath(index, 'protocol')">
                    <RadioGroup :name="`protocol-${index}`" v-model:value="localFields[index].protocol">
                        <Radio value="http">http</Radio>
                        <Radio value="https">https</Radio>
                    </RadioGroup>
                </FormItem>
                </Col>
            </Row>
            <Row v-show="localFields[index].proxy">
                <Col :span="24">
                <FormItem label="Connection Timeout" :label-col="{ span: 3 }"
                    :name="getFieldPath(index, 'connection_timeout')">
                    <InputNumber v-model:value="localFields[index].connection_timeout" placeholder="5" />
                </FormItem>
                </Col>
            </Row>
            <Row v-show="localFields[index].proxy">
                <Col :span="24">
                <FormItem label="Read Timeout" :label-col="{ span: 3 }" :name="getFieldPath(index, 'read_timeout')">
                    <InputNumber v-model:value="localFields[index].read_timeout" placeholder="5" />
                </FormItem>
                </Col>
            </Row>
            <Row v-show="localFields[index].proxy">
                <Col :span="24">
                <FormItem label="Write Timeout" :label-col="{ span: 3 }" :name="getFieldPath(index, 'write_timeout')">
                    <InputNumber v-model:value="localFields[index].write_timeout" placeholder="5" />
                </FormItem>
                </Col>
            </Row>
            <Row v-show="localFields[index].proxy">
                <Col :span="24">
                <FormItem label="Idle Timeout" :label-col="{ span: 3 }" :name="getFieldPath(index, 'idle_timeout')">
                    <InputNumber v-model:value="localFields[index].idle_timeout" placeholder="30" />
                </FormItem>
                </Col>
            </Row>
            <Row v-show="localFields[index].proxy">
                <Col :span="24">
                <FormItem label="Sni" :label-col="{ span: 3 }" :name="getFieldPath(index, 'sni')">
                    <Input v-model:value="localFields[index].sni" placeholder="" />
                </FormItem>
                </Col>
            </Row>
            <Row v-show="localFields[index].proxy">
                <Col :span="24">
                <FormItem label="Client Max Body Size" :label-col="{ span: 3 }"
                    :name="getFieldPath(index, 'client_max_body_size')">
                    <InputNumber v-model:value="localFields[index].client_max_body_size" placeholder="0" />
                </FormItem>
                </Col>
            </Row>
            <Row v-show="localFields[index].proxy">
                <Col :span="24">
                <FormItem label="Rewrite" :label-col="{ span: 3 }" :name="getFieldPath(index, 'rewrite')">
                    <Input v-model:value="localFields[index].rewrite" placeholder="Example: ^/(.*) /v2/api/$1" />
                </FormItem>
                </Col>
            </Row>
            <Row v-show="localFields[index].proxy">
                <Col :span="24">
                <FormItem label="Upstream" :label-col="{ span: 3 }" :name="getFieldPath(index, 'upstream')"
                    extra="Example: ip:port, enter each IP on a new line.">
                    <Textarea v-model:value="localFields[index].upstream" :rows="5" placeholder="Enter something..." />
                </FormItem>
                </Col>
            </Row>
            <Row v-show="!localFields[index].proxy">
                <Col :span="24">
                <FormItem label="Root Directory" :label-col="{ span: 3 }" :name="getFieldPath(index, 'root_dir')">
                    <Input v-model:value="localFields[index].root_dir" placeholder="" />
                </FormItem>
                </Col>
            </Row>
            <Row v-show="!localFields[index].proxy">
                <Col :span="24">
                <FormItem label="Auto Index" :label-col="{ span: 3 }" :name="getFieldPath(index, 'auto_index')">
                    <Switch v-model:checked="localFields[index].auto_index" />
                </FormItem>
                </Col>
            </Row>

            <Divider v-show="localFields[index].proxy" />
            <Row v-show="localFields[index].proxy">
                <Col :span="24">
                <DynamicHeader label="Add headers" :min="1" :max="10"
                    :namePath="getFieldPath(index, 'proxy_add_headers')"
                    v-model:modelValue="localFields[index].proxy_add_headers" />
                </Col>
            </Row>

            <Divider v-show="localFields[index].proxy" />
            <Row v-show="localFields[index].proxy">
                <Col :span="24">
                <DynamicHeader label="Set headers" :min="1" :max="10"
                    :namePath="getFieldPath(index, 'proxy_set_headers')"
                    v-model:modelValue="localFields[index].proxy_set_headers" />
                </Col>
            </Row>

        </Card>

    </div>

</template>
