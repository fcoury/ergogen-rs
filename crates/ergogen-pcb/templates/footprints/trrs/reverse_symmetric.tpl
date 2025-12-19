(module TRRS-PJ-320A-dual (layer F.Cu) (tedit 5970F8E5)

      (at {{at}})   

      
      (fp_text reference "{{ref}}" (at 0 14.2) (layer Dwgs.User) (effects (font (size 1 1) (thickness 0.15))))
      (fp_text value TRRS-PJ-320A-dual (at 0 -5.6) (layer F.Fab) (effects (font (size 1 1) (thickness 0.15))))

      
      (fp_line (start 0.5 -2) (end -5.1 -2) (layer Dwgs.User) (width 0.15))
      (fp_line (start -5.1 0) (end -5.1 -2) (layer Dwgs.User) (width 0.15))
      (fp_line (start 0.5 0) (end 0.5 -2) (layer Dwgs.User) (width 0.15))
      (fp_line (start -5.35 0) (end -5.35 12.1) (layer Dwgs.User) (width 0.15))
      (fp_line (start 0.75 0) (end 0.75 12.1) (layer Dwgs.User) (width 0.15))
      (fp_line (start 0.75 12.1) (end -5.35 12.1) (layer Dwgs.User) (width 0.15))
      (fp_line (start 0.75 0) (end -5.35 0) (layer Dwgs.User) (width 0.15))

      
        
        (pad "" np_thru_hole circle (at -2.3 8.6) (size 1.5 1.5) (drill 1.5) (layers *.Cu *.Mask))
        (pad "" np_thru_hole circle (at -2.3 1.6) (size 1.5 1.5) (drill 1.5) (layers *.Cu *.Mask))
      
        
        (pad 1 thru_hole oval (at 0 11.3 0) (size 1.6 2.2) (drill oval 0.9 1.5) (layers *.Cu *.Mask) (net {{net_A_id}} "{{net_A}}"))
        (pad 2 thru_hole oval (at -4.6 10.2 0) (size 1.6 2.2) (drill oval 0.9 1.5) (layers *.Cu *.Mask) (net {{net_B_id}} "{{net_B}}"))
        (pad 3 thru_hole oval (at -4.6 6.2 0) (size 1.6 2.2) (drill oval 0.9 1.5) (layers *.Cu *.Mask) (net {{net_C_id}} "{{net_C}}"))
        (pad 4 thru_hole oval (at -4.6 3.2 0) (size 1.6 2.2) (drill oval 0.9 1.5) (layers *.Cu *.Mask) (net {{net_D_id}} "{{net_D}}"))
      
        
        (pad 1 thru_hole oval (at -4.6 11.3 0) (size 1.6 2.2) (drill oval 0.9 1.5) (layers *.Cu *.Mask) (net {{net_A_id}} "{{net_A}}"))
        (pad 2 thru_hole oval (at 0 10.2 0) (size 1.6 2.2) (drill oval 0.9 1.5) (layers *.Cu *.Mask) (net {{net_B_id}} "{{net_B}}"))
        (pad 3 thru_hole oval (at 0 6.2 0) (size 1.6 2.2) (drill oval 0.9 1.5) (layers *.Cu *.Mask) (net {{net_C_id}} "{{net_C}}"))
        (pad 4 thru_hole oval (at 0 3.2 0) (size 1.6 2.2) (drill oval 0.9 1.5) (layers *.Cu *.Mask) (net {{net_D_id}} "{{net_D}}"))
      )