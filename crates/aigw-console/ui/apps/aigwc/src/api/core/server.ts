
export namespace ServerTableApi {
  export interface PageFetchParams {
    [key: string]: any;
    page: number;
    page_size: number;
  }
}

import { requestClient } from '#/api/request';

export async function getServerTableApi(cluster: string | null, params: ServerTableApi.PageFetchParams) {
  return requestClient.get('/servers/' + cluster, { params });
}