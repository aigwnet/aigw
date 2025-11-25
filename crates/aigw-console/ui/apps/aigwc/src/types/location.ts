export interface LocationItem {
    path: string
    proxy: boolean
    protocol: string
    connection_timeout: number
    read_timeout: number
    write_timeout: number
    idle_timeout: number
    sni: string
    client_max_body_size: number
    rewrite: string
    http_version: string
    upstream: string
    root_dir: string
    auto_index: boolean
    proxy_add_headers: Array<{ name: string; value: string }>
    proxy_set_headers: Array<{ name: string; value: string }>
    proxy_remove_headers: Array<{ name: string }>
    response_add_headers: Array<{ name: string; value: string }>
    response_set_headers: Array<{ name: string; value: string }>
    response_remove_headers: Array<{ name: string }>
}

export const defaultLocationItem = (): LocationItem => ({
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
    http_version: "",
    upstream: "",
    root_dir: "",
    auto_index: false,
    proxy_add_headers: [{ name: "", value: "" }],
    proxy_set_headers: [{ name: "", value: "" }],
    proxy_remove_headers: [{ name: "" }],
    response_add_headers: [{ name: "", value: "" }],
    response_set_headers: [{ name: "", value: "" }],
    response_remove_headers: [{ name: "" }],
});