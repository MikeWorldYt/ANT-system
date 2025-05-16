import { NgClass } from '@angular/common';
import { Component, OnInit } from '@angular/core';
import { MenuService } from '../../services/menu.service';
import { LanguageDropdownComponent } from '../../features/language-dropdown/language-dropdown.component';

@Component({
  selector: 'app-header',
  standalone: true,
  imports: [
    NgClass,
    LanguageDropdownComponent
  ],
  templateUrl: './header.component.html',
  styleUrl: './header.component.css'
})
export class HeaderComponent implements OnInit{
  menu: boolean = true;
  constructor(private menuService: MenuService) {}

  ngOnInit() {
    this.menuService.currentMenu.subscribe(menu => this.menu = menu);
  }

}
