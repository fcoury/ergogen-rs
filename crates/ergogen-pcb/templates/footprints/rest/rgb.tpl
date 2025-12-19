(module WS2812B (layer F.Cu) (tedit 53BEE615)

            (at {{at}})

            
            (fp_text reference "{{ref}}" (at 0 0) (layer F.SilkS) hide (effects (font (size 1.27 1.27) (thickness 0.15))))
            (fp_text value "" (at 0 0) (layer F.SilkS) hide (effects (font (size 1.27 1.27) (thickness 0.15))))

            (fp_line (start -1.75 -1.75) (end -1.75 1.75) (layer F.SilkS) (width 0.15))
            (fp_line (start -1.75 1.75) (end 1.75 1.75) (layer F.SilkS) (width 0.15))
            (fp_line (start 1.75 1.75) (end 1.75 -1.75) (layer F.SilkS) (width 0.15))
            (fp_line (start 1.75 -1.75) (end -1.75 -1.75) (layer F.SilkS) (width 0.15))

            (fp_line (start -2.5 -2.5) (end -2.5 2.5) (layer F.SilkS) (width 0.15))
            (fp_line (start -2.5 2.5) (end 2.5 2.5) (layer F.SilkS) (width 0.15))
            (fp_line (start 2.5 2.5) (end 2.5 -2.5) (layer F.SilkS) (width 0.15))
            (fp_line (start 2.5 -2.5) (end -2.5 -2.5) (layer F.SilkS) (width 0.15))

            (fp_poly (pts (xy 4 2.2) (xy 4 0.375) (xy 5 1.2875)) (layer F.SilkS) (width 0.1))

            (pad 1 smd rect (at -2.2 -0.875 0) (size 2.6 1) (layers F.Cu F.Paste F.Mask) (net {{net_VCC_id}} "{{net_VCC}}"))
            (pad 2 smd rect (at -2.2 0.875 0) (size 2.6 1) (layers F.Cu F.Paste F.Mask) (net {{net_dout_id}} "{{net_dout}}"))
            (pad 3 smd rect (at 2.2 0.875 0) (size 2.6 1) (layers F.Cu F.Paste F.Mask) (net {{net_GND_id}} "{{net_GND}}"))
            (pad 4 smd rect (at 2.2 -0.875 0) (size 2.6 1) (layers F.Cu F.Paste F.Mask) (net {{net_din_id}} "{{net_din}}"))

            (pad 11 smd rect (at -2.5 -1.6 0) (size 2 1.2) (layers F.Cu F.Paste F.Mask) (net {{net_VCC_id}} "{{net_VCC}}"))
            (pad 22 smd rect (at -2.5 1.6 0) (size 2 1.2) (layers F.Cu F.Paste F.Mask) (net {{net_dout_id}} "{{net_dout}}"))
            (pad 33 smd rect (at 2.5 1.6 0) (size 2 1.2) (layers F.Cu F.Paste F.Mask) (net {{net_GND_id}} "{{net_GND}}"))
            (pad 44 smd rect (at 2.5 -1.6 0) (size 2 1.2) (layers F.Cu F.Paste F.Mask) (net {{net_din_id}} "{{net_din}}"))
            
        )