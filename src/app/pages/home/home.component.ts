import { Component } from '@angular/core';
import { Router, RouterOutlet } from '@angular/router';
import { HeaderComponent } from '../../layout/header/header.component';
import { SidebarComponent } from '../../layout/sidebar/sidebar.component';
import { MainComponent } from '../../layout/main/main.component';
import { FooterComponent } from '../../layout/footer/footer.component';
import { NgClass } from '@angular/common';
import { MenuService } from '../../services/menu.service';

@Component({
  selector: 'app-home',
  standalone: true,
  imports: [
      NgClass,
      RouterOutlet,
      HeaderComponent,
      SidebarComponent,
      MainComponent,
      FooterComponent
    ],
  templateUrl: './home.component.html',
  styleUrl: './home.component.css'
})
export default class HomeComponent {
  // Menu
    menu: boolean;
    constructor(private menuService: MenuService) {
      this.menu = window.innerWidth > 768 ? true : false;
    }

  showMenu() {
    this.menu = !this.menu;
    this.menuService.changeMenu(this.menu);
  }

}