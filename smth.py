import torch


def solution(q_a, scale_a, sf_g_a: float, q_x, scale_x, sf_g_x: float, y, m: int, k: int):
    del m, k
    from flashinfer.fp4_quantization import e2m1_and_ufp8sf_scale_to_float

    with torch.no_grad():
        sf_a_dec = torch.tensor([1.0 / sf_g_a], device=q_a.device, dtype=torch.float32)
        sf_x_dec = torch.tensor([1.0 / sf_g_x], device=q_x.device, dtype=torch.float32)
        a_deq = e2m1_and_ufp8sf_scale_to_float(q_a, scale_a, sf_a_dec).float()
        x_deq = e2m1_and_ufp8sf_scale_to_float(q_x, scale_x, sf_x_dec).float().squeeze(0)
        y.copy_(torch.matmul(a_deq, x_deq).to(y.dtype))