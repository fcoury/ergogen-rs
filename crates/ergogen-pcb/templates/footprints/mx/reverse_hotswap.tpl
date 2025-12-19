(module MX (layer F.Cu) (tedit 5DD4F656)
      (at {{at}})

      
      (fp_text reference "{{ref}}" (at 0 0) (layer F.SilkS) hide (effects (font (size 1.27 1.27) (thickness 0.15))))
      (fp_text value "" (at 0 0) (layer F.SilkS) hide (effects (font (size 1.27 1.27) (thickness 0.15))))

      
      (fp_line (start -7 -6) (end -7 -7) (layer Dwgs.User) (width 0.15))
      (fp_line (start -7 7) (end -6 7) (layer Dwgs.User) (width 0.15))
      (fp_line (start -6 -7) (end -7 -7) (layer Dwgs.User) (width 0.15))
      (fp_line (start -7 7) (end -7 6) (layer Dwgs.User) (width 0.15))
      (fp_line (start 7 6) (end 7 7) (layer Dwgs.User) (width 0.15))
      (fp_line (start 7 -7) (end 6 -7) (layer Dwgs.User) (width 0.15))
      (fp_line (start 6 7) (end 7 7) (layer Dwgs.User) (width 0.15))
      (fp_line (start 7 -7) (end 7 -6) (layer Dwgs.User) (width 0.15))
    
      
      (pad "" np_thru_hole circle (at 0 0) (size 3.9878 3.9878) (drill 3.9878) (layers *.Cu *.Mask))

      
      (pad "" np_thru_hole circle (at 5.08 0) (size 1.7018 1.7018) (drill 1.7018) (layers *.Cu *.Mask))
      (pad "" np_thru_hole circle (at -5.08 0) (size 1.7018 1.7018) (drill 1.7018) (layers *.Cu *.Mask))
      
        
        
        
        (pad "" np_thru_hole circle (at 2.54 -5.08) (size 3 3) (drill 3) (layers *.Cu *.Mask))
        (pad "" np_thru_hole circle (at -3.81 -2.54) (size 3 3) (drill 3) (layers *.Cu *.Mask))
        
        
        (pad 1 smd rect (at -7.085 -2.54 0) (size 2.55 2.5) (layers B.Cu B.Paste B.Mask) (net {{net_from_id}} "{{net_from}}"))
        (pad 2 smd rect (at 5.842 -5.08 0) (size 2.55 2.5) (layers B.Cu B.Paste B.Mask) (net {{net_to_id}} "{{net_to}}"))
        
        
        
        (pad "" np_thru_hole circle (at -2.54 -5.08) (size 3 3) (drill 3) (layers *.Cu *.Mask))
        (pad "" np_thru_hole circle (at 3.81 -2.54) (size 3 3) (drill 3) (layers *.Cu *.Mask))
        
        
        (pad 1 smd rect (at 7.085 -2.54 0) (size 2.55 2.5) (layers F.Cu F.Paste F.Mask) (net {{net_from_id}} "{{net_from}}"))
        (pad 2 smd rect (at -5.842 -5.08 0) (size 2.55 2.5) (layers F.Cu F.Paste F.Mask) (net {{net_to_id}} "{{net_to}}"))
        )